using System.Runtime.CompilerServices;

// パッケージの中のアセンブリにだけ internal を見せる。アプリに見せるものは public にする
[assembly: InternalsVisibleTo("VolTech.Client.Editor")]
[assembly: InternalsVisibleTo("VolTech.Client.Tests.EditMode")]
