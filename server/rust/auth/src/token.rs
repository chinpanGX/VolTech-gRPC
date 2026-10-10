/// アクセストークンにする、256 ビットのランダムな値の 16 進文字列を作る。
pub(crate) fn generate() -> String {
    let bytes: [u8; 32] = rand::random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
