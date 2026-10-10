using System;
using System.Collections.Generic;

namespace VolTech.Client
{
    /// <summary>
    /// 基盤の予約エラー(ReservedErrors)と、アプリのエラーの定義ファイルから生成した一覧から、既定の action を引く。
    /// </summary>
    /// <example>
    /// <code>new ErrorPolicyProvider(AppErrors.All)</code>
    /// </example>
    public sealed class ErrorPolicyProvider : IErrorPolicyProvider
    {
        private readonly Dictionary<ErrorCode, ErrorAction> defaultActions = new();

        /// <summary>
        /// アプリのエラーの一覧を受け取って作る。基盤の予約エラーは自動で入る。
        /// </summary>
        /// <exception cref="ArgumentException">アプリのエラーが予約エラーの domain を使っている、または重複している。</exception>
        public ErrorPolicyProvider(IEnumerable<ErrorDefinition> appErrors)
        {
            foreach (var definition in ReservedErrors.All)
            {
                defaultActions.Add(definition.Code, definition.DefaultAction);
            }

            foreach (var definition in appErrors ?? Array.Empty<ErrorDefinition>())
            {
                // 生成した一覧なら起きないが、手で作った一覧で予約エラーを上書きさせない
                if (definition.Code.IsReserved)
                {
                    throw new ArgumentException($"アプリのエラーに予約エラーの domain は使えない: {definition.Code}", nameof(appErrors));
                }

                if (!defaultActions.TryAdd(definition.Code, definition.DefaultAction))
                {
                    throw new ArgumentException($"エラーコードが重複している: {definition.Code}", nameof(appErrors));
                }
            }
        }

        /// <summary>
        /// 知っているエラーコードかを返す。
        /// </summary>
        public bool IsKnown(ErrorCode code)
        {
            return defaultActions.ContainsKey(code);
        }

        /// <summary>
        /// エラーコードの既定の action を返す。知らないコードは Title。
        /// </summary>
        public ErrorAction GetDefaultAction(ErrorCode code)
        {
            return defaultActions.TryGetValue(code, out var action) && action != ErrorAction.Unspecified
                ? action
                : ErrorAction.Title;
        }
    }
}
