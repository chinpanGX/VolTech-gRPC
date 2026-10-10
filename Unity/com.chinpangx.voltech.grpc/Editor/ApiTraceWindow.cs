using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using UnityEditor;
using UnityEditor.Networking.PlayerConnection;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.Networking.PlayerConnection;
using UnityEngine.UIElements;
using VolTech.Client.Tracing;

namespace VolTech.Client.Editor
{
    /// <summary>
    /// 通信の Viewer。API のリクエストとレスポンスを1件ずつ、ヘッダーと Body(JSON)に分けて見る。
    /// </summary>
    /// <remarks>
    /// UI Toolkit で、USS・UXML は使わずに C# だけで組み立てる。見るだけで、送り直しはしない(アプリの操作で送る)。
    /// 「Editor」はエディタで動かした通信(ApiTraceStore.Shared)、「Player」は USB などでつないだ実機の通信(ApiTraceReceiver.Store)。
    /// </remarks>
    public sealed class ApiTraceWindow : EditorWindow
    {
        private const float HeaderKeyWidth = 180;
        private const long RefreshIntervalMs = 200;

        // 詳細の Body の TextField に出す文字数の上限
        private const int MaxShownBodyLength = 64 * 1024;

        private static readonly Color errorColor = new(0.95f, 0.45f, 0.4f);
        private static readonly Color pendingColor = new(0.6f, 0.6f, 0.6f);
        private static readonly Color keyColor = new(0.65f, 0.75f, 0.9f);

        // ドメインのリロードをまたいで残す設定
        [SerializeField]
        private TraceSource source = TraceSource.Editor;

        [SerializeField]
        private bool followLatest = true;

        private readonly List<ApiTrace> shown = new();
        private ListView list;
        private ScrollView detail;
        private IMGUIContainer connectionDropdown;
        private ToolbarToggle recordToggle;
        private IConnectionState connectionState;
        private string filter = "";
        private int shownVersion = -1;
        private ApiTraceStore shownStore;
        private int selectedId = -1;
        private ApiTrace detailTrace;

        private enum TraceSource
        {
            Editor,
            Player,
        }

        private ApiTraceStore Store => source == TraceSource.Editor ? ApiTraceStore.Shared : ApiTraceReceiver.Store;

        [MenuItem("Window/VolTech gRPC/Request Viewer")]
        private static void Open()
        {
            GetWindow<ApiTraceWindow>("Request Viewer");
        }

        private void OnEnable()
        {
            connectionState = PlayerConnectionGUIUtility.GetConnectionState(this);
        }

        private void OnDisable()
        {
            connectionState?.Dispose();
            connectionState = null;
        }

        private void CreateGUI()
        {
            rootVisualElement.Add(CreateToolbar());

            var split = new TwoPaneSplitView(0, 380, TwoPaneSplitViewOrientation.Horizontal);
            split.style.flexGrow = 1;
            split.Add(CreateList());
            detail = new ScrollView(ScrollViewMode.Vertical);
            detail.style.paddingLeft = 8;
            detail.style.paddingRight = 8;
            detail.style.paddingTop = 4;
            split.Add(detail);
            rootVisualElement.Add(split);

            UpdateSourceControls();
            ShowDetail(null);
            // 記録は別スレッドから足されるので、UI はここで定期的に版を見て描き直す
            rootVisualElement.schedule.Execute(Refresh).Every(RefreshIntervalMs);
        }

        private Toolbar CreateToolbar()
        {
            var toolbar = new Toolbar();

            var clear = new ToolbarButton(() => Store.Clear()) { text = "Clear" };
            toolbar.Add(clear);

            recordToggle = new ToolbarToggle { text = "Record", value = ApiTraceStore.Shared.Recording };
            recordToggle.RegisterValueChangedCallback(change => ApiTraceStore.Shared.Recording = change.newValue);
            toolbar.Add(recordToggle);

            var clearOnPlayToggle = new ToolbarToggle { text = "Clear on Play", value = ApiTraceReceiver.ClearOnPlay };
            clearOnPlayToggle.RegisterValueChangedCallback(change => ApiTraceReceiver.ClearOnPlay = change.newValue);
            toolbar.Add(clearOnPlayToggle);

            var followToggle = new ToolbarToggle { text = "Follow latest", value = followLatest };
            followToggle.RegisterValueChangedCallback(change => followLatest = change.newValue);
            toolbar.Add(followToggle);

            toolbar.Add(new ToolbarSpacer());

            var sourceMenu = new ToolbarMenu { text = source.ToString() };
            foreach (var value in (TraceSource[])Enum.GetValues(typeof(TraceSource)))
            {
                sourceMenu.menu.AppendAction(
                    value.ToString(),
                    _ =>
                    {
                        source = value;
                        sourceMenu.text = value.ToString();
                        UpdateSourceControls();
                    },
                    _ => source == value ? DropdownMenuAction.Status.Checked : DropdownMenuAction.Status.Normal);
            }

            toolbar.Add(sourceMenu);

            // 接続先の端末の選択は、Console・Profiler と同じ Unity の部品を使う(IMGUI にしか無いので、IMGUIContainer に入れる)
            connectionDropdown = new IMGUIContainer(() =>
                PlayerConnectionGUILayout.ConnectionTargetSelectionDropdown(connectionState, EditorStyles.toolbarDropDown));
            connectionDropdown.style.width = 220;
            toolbar.Add(connectionDropdown);

            var spacer = new VisualElement();
            spacer.style.flexGrow = 1;
            toolbar.Add(spacer);

            var search = new ToolbarSearchField();
            search.RegisterValueChangedCallback(change =>
            {
                filter = change.newValue ?? "";
                shownVersion = -1;
            });
            toolbar.Add(search);

            return toolbar;
        }

        private ListView CreateList()
        {
            list = new ListView
            {
                fixedItemHeight = 20,
                selectionType = SelectionType.Single,
                itemsSource = shown,
                makeItem = MakeRow,
                bindItem = (element, index) => BindRow(element, shown[index]),
            };
            list.style.flexGrow = 1;
            list.selectionChanged += selected =>
            {
                var trace = selected.OfType<ApiTrace>().FirstOrDefault();
                selectedId = trace?.Id ?? -1;
                ShowDetail(trace);
            };
            return list;
        }

        // 一覧の1行: 時刻・RPC の短い名前・状態・所要時間
        private static VisualElement MakeRow()
        {
            var row = new VisualElement();
            row.style.flexDirection = FlexDirection.Row;
            row.style.alignItems = Align.Center;
            row.style.paddingLeft = 4;
            row.Add(RowCell("time", 90));
            row.Add(RowCell("method", 0));
            row.Add(RowCell("state", 150));
            row.Add(RowCell("duration", 60));
            return row;
        }

        private static Label RowCell(string name, float width)
        {
            var label = new Label { name = name };
            if (width > 0)
            {
                label.style.width = width;
            }
            else
            {
                label.style.flexGrow = 1;
            }

            label.style.overflow = Overflow.Hidden;
            label.style.textOverflow = TextOverflow.Ellipsis;
            return label;
        }

        private static void BindRow(VisualElement row, ApiTrace trace)
        {
            row.Q<Label>("time").text = FormatTime(trace.StartedAt, "HH:mm:ss");
            row.Q<Label>("method").text = trace.ShortMethod;
            row.Q<Label>("state").text = StateText(trace);
            row.Q<Label>("duration").text = trace.DurationMs < 0 ? "" : $"{trace.DurationMs:0} ms";
            var color = trace.State switch
            {
                ApiTraceState.Failed => errorColor,
                ApiTraceState.Pending => pendingColor,
                _ => (StyleColor)StyleKeyword.Null,
            };
            foreach (var label in row.Query<Label>().ToList())
            {
                label.style.color = color;
            }
        }

        private void Refresh()
        {
            var store = Store;
            if (store == shownStore && store.Version == shownVersion)
            {
                return;
            }

            shownStore = store;
            shownVersion = store.Version;
            shown.Clear();
            shown.AddRange(store.Snapshot().Where(trace =>
                string.IsNullOrEmpty(filter) || trace.Method.IndexOf(filter, StringComparison.OrdinalIgnoreCase) >= 0));
            list.RefreshItems();

            var index = followLatest && shown.Count > 0
                ? shown.Count - 1
                : shown.FindIndex(trace => trace.Id == selectedId);
            if (index >= 0)
            {
                list.SetSelectionWithoutNotify(new[] { index });
                list.ScrollToItem(index);
                // 送信中だった記録が埋まったら、詳細も描き直す
                if (!ReferenceEquals(shown[index], detailTrace))
                {
                    selectedId = shown[index].Id;
                    ShowDetail(shown[index]);
                }
            }
            else
            {
                list.ClearSelection();
                ShowDetail(null);
            }
        }

        private void ShowDetail(ApiTrace trace)
        {
            detailTrace = trace;
            detail.Clear();
            if (trace == null)
            {
                detail.Add(new Label("通信を選ぶと、ヘッダーと Body を表示します。"));
                return;
            }

            var title = new Label(trace.Method);
            title.style.unityFontStyleAndWeight = FontStyle.Bold;
            title.selection.isSelectable = true;
            detail.Add(title);
            var duration = trace.DurationMs < 0 ? "" : $"  {trace.DurationMs:0} ms";
            var truncated = trace.Truncated ? "  (Body truncated)" : "";
            detail.Add(new Label($"{FormatTime(trace.StartedAt, "yyyy-MM-dd HH:mm:ss.fff")}{duration}  {StateText(trace)}{truncated}"));

            var request = Section("Request");
            request.Add(HeadersSection("Headers", trace.RequestHeaders));
            request.Add(BodySection(trace.RequestBody));
            detail.Add(request);

            var response = Section("Response");
            if (trace.State == ApiTraceState.Pending)
            {
                response.Add(new Label("応答を待っています。"));
            }
            else
            {
                response.Add(HeadersSection("Headers", trace.ResponseHeaders));
                response.Add(BodySection(trace.ResponseBody));
                response.Add(HeadersSection("Trailers", trace.Trailers));
            }

            detail.Add(response);

            if (trace.State == ApiTraceState.Failed)
            {
                var error = Section("Error");
                var rows = new List<ApiTraceHeader>
                {
                    new("status", trace.StatusCode),
                    new("detail", trace.StatusDetail),
                };
                if (!string.IsNullOrEmpty(trace.ErrorReason))
                {
                    rows.Add(new ApiTraceHeader("reason", trace.ErrorReason));
                    rows.Add(new ApiTraceHeader("domain", trace.ErrorDomain));
                }

                error.Add(HeaderRows(rows));
                error.Add(HeadersSection("Metadata", trace.ErrorMetadata));
                detail.Add(error);
            }
        }

        private static Foldout Section(string text)
        {
            var foldout = new Foldout { text = text, value = true };
            foldout.style.marginTop = 6;
            foldout.Q<Toggle>().style.unityFontStyleAndWeight = FontStyle.Bold;
            return foldout;
        }

        // ヘッダーは項目ごとに1行(名前の Label と値の Label)
        private static Foldout HeadersSection(string text, List<ApiTraceHeader> headers)
        {
            var foldout = new Foldout { text = $"{text} ({headers?.Count ?? 0})", value = true };
            foldout.Add(HeaderRows(headers));
            return foldout;
        }

        private static VisualElement HeaderRows(List<ApiTraceHeader> headers)
        {
            var rows = new VisualElement();
            if (headers == null || headers.Count == 0)
            {
                rows.Add(new Label("(none)"));
                return rows;
            }

            foreach (var header in headers)
            {
                var row = new VisualElement();
                row.style.flexDirection = FlexDirection.Row;
                row.style.marginBottom = 1;

                var key = new Label(header.Key);
                key.style.width = HeaderKeyWidth;
                key.style.flexShrink = 0;
                key.style.color = keyColor;
                key.selection.isSelectable = true;
                row.Add(key);

                // 値は選んで一部だけコピーできるようにする
                var value = new Label(header.Value);
                value.style.flexGrow = 1;
                value.style.flexShrink = 1;
                value.style.whiteSpace = WhiteSpace.Normal;
                value.selection.isSelectable = true;
                row.Add(value);

                rows.Add(row);
            }

            return rows;
        }

        // Body は読み取り専用の複数行の TextField に JSON を出し、Copy ボタンで全体をコピーできるようにする
        private static VisualElement BodySection(string body)
        {
            var section = new VisualElement();
            section.style.marginTop = 2;

            var header = new VisualElement();
            header.style.flexDirection = FlexDirection.Row;
            header.style.alignItems = Align.Center;
            var label = new Label("Body");
            label.style.flexGrow = 1;
            header.Add(label);
            var copy = new Button(() => EditorGUIUtility.systemCopyBuffer = body ?? "") { text = "Copy" };
            copy.SetEnabled(!string.IsNullOrEmpty(body));
            header.Add(copy);
            section.Add(header);

            // 大きな Body をそのまま TextField に入れるとエディタが固まるので、表示は先頭だけにする(Copy は全体)
            var shownBody = string.IsNullOrEmpty(body) ? "" : body;
            if (shownBody.Length > MaxShownBodyLength)
            {
                shownBody = shownBody.Substring(0, MaxShownBodyLength)
                    + $"\n... (表示は先頭 {MaxShownBodyLength} 文字まで。全体は Copy でコピーする)";
            }

            var text = new TextField
            {
                multiline = true,
                isReadOnly = true,
                value = shownBody,
            };
            text.style.whiteSpace = WhiteSpace.Normal;
            section.Add(text);
            return section;
        }

        private void UpdateSourceControls()
        {
            if (connectionDropdown == null)
            {
                return;
            }

            // 接続先の選択は、実機の通信を見るときだけ出す。記録の一時停止は、エディタの通信にだけ効く
            connectionDropdown.style.display = source == TraceSource.Player ? DisplayStyle.Flex : DisplayStyle.None;
            recordToggle.style.display = source == TraceSource.Editor ? DisplayStyle.Flex : DisplayStyle.None;
            selectedId = -1;
            shownVersion = -1;
        }

        private static string StateText(ApiTrace trace)
        {
            return trace.State switch
            {
                ApiTraceState.Pending => "Pending",
                ApiTraceState.Succeeded => "OK",
                _ => string.IsNullOrEmpty(trace.ErrorReason) ? trace.StatusCode : trace.ErrorReason,
            };
        }

        private static string FormatTime(string iso, string format)
        {
            return DateTime.TryParse(iso, CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind, out var time)
                ? time.ToString(format, CultureInfo.InvariantCulture)
                : iso;
        }
    }
}
