using Google.Protobuf;
using Google.Protobuf.Collections;
using NUnit.Framework;

namespace VolTech.Client.Tests.EditMode
{
    /// <summary>
    /// パッケージが依存する Google.Protobuf の版が、Google.Api.CommonProtos の参照する版と一致していることを確かめる。
    /// エディタ(6000.6 以降)は別の版の Google.Protobuf を同梱していて、版が一致しないと CommonProtos がそちらに結び付き、
    /// ErrorInfo を読むところで MissingMethodException になる(業務エラーがすべて不明なエラーになる)。
    /// </summary>
    [Description("パッケージが依存する Google.Protobuf の版が、Google.Api.CommonProtos の参照する版と一致していることを確かめる")]
    public class ProtobufVersionTests
    {
        /// <summary>
        /// CommonProtos が参照する Google.Protobuf の版が、パッケージのコードが使う Google.Protobuf の版と同じ。
        /// 違うときは、package.json の org.nuget.google.protobuf を CommonProtos の依存と同じ版にする。
        /// </summary>
        [Test]
        [Description("CommonProtos が参照する Google.Protobuf の版が、パッケージのコードが使う版と同じ")]
        public void CommonProtosReferencesSameProtobufVersion()
        {
            var used = typeof(IMessage).Assembly.GetName().Version;
            var referenced = System.Array.Find(
                typeof(Google.Rpc.ErrorInfo).Assembly.GetReferencedAssemblies(),
                name => name.Name == "Google.Protobuf");

            Assert.That(referenced, Is.Not.Null);
            Assert.That(referenced.Version, Is.EqualTo(used),
                "package.json の org.nuget.google.protobuf を、org.nuget.google.api.commonprotos が依存する版にそろえる");
        }

        /// <summary>
        /// ErrorInfo.metadata の型が、パッケージのコードが使う Google.Protobuf の MapField と同じ型になる。
        /// </summary>
        [Test]
        [Description("ErrorInfo.metadata の型が、パッケージのコードが使う Google.Protobuf の MapField と同じ型になる")]
        public void ErrorInfoMetadataUsesSameProtobuf()
        {
            var metadataType = typeof(Google.Rpc.ErrorInfo).GetProperty(nameof(Google.Rpc.ErrorInfo.Metadata)).PropertyType;

            Assert.That(metadataType, Is.EqualTo(typeof(MapField<string, string>)));
        }
    }
}
