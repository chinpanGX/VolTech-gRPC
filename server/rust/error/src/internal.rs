use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::panic::Location;

use tonic::{Code, Status};

use crate::{reserved, with_code};

type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// サーバー内部エラー。元のエラー・処理の説明・発生した場所を持ち、`Status` にするときに原因をたどってログに出す。
///
/// `std::error::Error` を実装したエラーから `?` で作れる。ハンドラーが返す `Status` には `?`・`From` でなる。
/// `tonic::Status`(業務エラーなど)も `?` で包めるが、`Status` に戻すときは INTERNAL にせず、元のまま返す。
// std::error::Error は実装しない。実装すると From<E: Error> と標準の From<T> for T がぶつかる(anyhow と同じ制約)
pub struct Internal {
    error: Option<BoxError>,
    context: Option<Cow<'static, str>>,
    location: &'static Location<'static>,
}

impl Internal {
    /// 元のエラーが無いときに、説明だけで作る。
    #[track_caller]
    pub fn msg(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            error: None,
            context: Some(message.into()),
            location: Location::caller(),
        }
    }

    /// `Box<dyn Error + Send + Sync>` から作る。`Box<dyn Error>` は `std::error::Error` を実装せず、`?` では作れないため。
    #[track_caller]
    pub fn from_boxed(error: BoxError) -> Self {
        Self {
            error: Some(error),
            context: None,
            location: Location::caller(),
        }
    }

    /// 何をしようとして失敗したかの説明を付ける。ログでは元のエラーの前に出る。
    pub fn context(mut self, context: impl Into<Cow<'static, str>>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// 発生した場所(`?` や `internal()` を書いた場所)。
    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }

    pub(crate) fn into_status(self) -> Status {
        let error = match self.error {
            Some(error) => match error.downcast::<Status>() {
                Ok(status) => return *status,
                Err(error) => Some(error),
            },
            None => None,
        };
        let internal = Self { error, ..self };
        tracing::error!(
            "exception.message" = %internal,
            "code.file.path" = internal.location.file(),
            "code.line.number" = internal.location.line(),
            "internal error"
        );
        with_code(
            Code::Internal,
            reserved::INTERNAL_ERROR,
            "internal error",
            HashMap::new(),
        )
    }
}

impl<E> From<E> for Internal
where
    E: Error + Send + Sync + 'static,
{
    #[track_caller]
    fn from(error: E) -> Self {
        Self::from_boxed(Box::new(error))
    }
}

impl From<Internal> for Status {
    fn from(internal: Internal) -> Self {
        internal.into_status()
    }
}

/// 説明と、元のエラーから原因の順に `: ` でつないで書く。
impl Display for Internal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        if let Some(context) = &self.context {
            f.write_str(context)?;
            separator = ": ";
        }
        let mut source: Option<&(dyn Error + 'static)> = match &self.error {
            Some(error) => Some(error.as_ref()),
            None => None,
        };
        while let Some(error) = source {
            write!(f, "{separator}{error}")?;
            separator = ": ";
            source = error.source();
        }
        Ok(())
    }
}

impl Debug for Internal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self} (at {})", self.location)
    }
}

/// `Result` のエラーを `Internal`・INTERNAL の `Status` にする。発生した場所は、このメソッドを呼んだ場所になる。
pub trait ResultExt<T> {
    /// エラーを INTERNAL の `Status` にする(`internal()` と同じ)。`.map_err(internal)` の代わりに使う。
    fn or_internal(self) -> Result<T, Status>;

    /// エラーを、何をしようとして失敗したかの説明を付けた `Internal` にする。
    fn context(self, context: impl Into<Cow<'static, str>>) -> Result<T, Internal>;
}

impl<T, E: Into<Internal>> ResultExt<T> for Result<T, E> {
    #[track_caller]
    fn or_internal(self) -> Result<T, Status> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(crate::internal(error)),
        }
    }

    #[track_caller]
    fn context(self, context: impl Into<Cow<'static, str>>) -> Result<T, Internal> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into().context(context)),
        }
    }
}
