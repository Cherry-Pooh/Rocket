use crate::{Request, Data};
use crate::handler::{Outcome, ErrorHandler};
use crate::http::{Method, MediaType};

/// Type of a static handler, which users annotate with Rocket's attribute.
pub type StaticHandler = for<'r> fn(&'r Request<'_>, Data) -> Outcome<'r>;

/// Information generated by the `route` attribute during codegen.
pub struct StaticRouteInfo {
    /// The route's name, i.e, the name of the function.
    pub name: &'static str,
    /// The route's method.
    pub method: Method,
    /// The route's path, without the base mount point.
    pub path: &'static str,
    /// The route's format, if any.
    pub format: Option<MediaType>,
    /// The route's handler, i.e, the annotated function.
    pub handler: StaticHandler,
    /// The route's rank, if any.
    pub rank: Option<isize>,
}

/// Information generated by the `catch` attribute during codegen.
pub struct StaticCatchInfo {
    /// The catcher's status code.
    pub code: u16,
    /// The catcher's handler, i.e, the annotated function.
    pub handler: ErrorHandler,
}
