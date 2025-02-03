#[derive(Debug)]
pub struct NotReady;

#[derive(Debug)]
pub struct AccessDenied;

#[derive(Debug, PartialEq)]
pub enum RadarError {
    NotReady,
    AccessBlocked,
}
impl From<NotReady> for RadarError {
    fn from(_: NotReady) -> Self {
        RadarError::NotReady
    }
}
impl From<AccessDenied> for RadarError {
    fn from(_: AccessDenied) -> Self {
        RadarError::AccessBlocked
    }
}
