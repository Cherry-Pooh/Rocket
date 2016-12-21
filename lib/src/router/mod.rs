mod collider;
mod route;

pub use self::collider::Collider;
pub use self::route::Route;

use std::collections::hash_map::HashMap;

use request::Request;
use http::Method;

// type Selector = (Method, usize);
type Selector = Method;

#[derive(Default)]
pub struct Router {
    routes: HashMap<Selector, Vec<Route>>, // using 'selector' for now
}

impl Router {
    pub fn new() -> Router {
        Router { routes: HashMap::new() }
    }

    pub fn add(&mut self, route: Route) {
        // let selector = (route.method, route.path.segment_count());
        let selector = route.method;
        self.routes.entry(selector).or_insert_with(|| vec![]).push(route);
    }

    // TODO: Make a `Router` trait with this function. Rename this `Router`
    // struct to something like `RocketRouter`. If that happens, returning a
    // `Route` structure is inflexible. Have it be an associated type.
    // FIXME: Figure out a way to get more than one route, i.e., to correctly
    // handle ranking.
    pub fn route<'b>(&'b self, req: &Request) -> Vec<&'b Route> {
        trace_!("Trying to route: {}", req);
        // let num_segments = req.uri.segment_count();
        // self.routes.get(&(req.method, num_segments)).map_or(vec![], |routes| {
        self.routes.get(&req.method()).map_or(vec![], |routes| {
            let mut matches: Vec<_> = routes.iter()
                .filter(|r| r.collides_with(req))
                .collect();

            // FIXME: Presort vector to avoid a sort on each route.
            matches.sort_by(|a, b| a.rank.cmp(&b.rank));
            trace_!("All matches: {:?}", matches);
            matches
        })
    }

    pub fn has_collisions(&self) -> bool {
        let mut result = false;
        for routes in self.routes.values() {
            for (i, a_route) in routes.iter().enumerate() {
                for b_route in routes.iter().skip(i + 1) {
                    if a_route.collides_with(b_route) {
                        result = true;
                        warn!("{} and {} collide!", a_route, b_route);
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod test {
    use super::{Router, Route};

    use http::Method;
    use http::Method::*;
    use http::uri::URI;
    use request::Request;
    use data::Data;
    use handler::Outcome;

    fn dummy_handler(_req: &Request, _: Data) -> Outcome<'static> {
        Outcome::of("hi")
    }

    fn router_with_routes(routes: &[&'static str]) -> Router {
        let mut router = Router::new();
        for route in routes {
            let route = Route::new(Get, route.to_string(), dummy_handler);
            router.add(route);
        }

        router
    }

    fn router_with_ranked_routes(routes: &[(isize, &'static str)]) -> Router {
        let mut router = Router::new();
        for &(rank, route) in routes {
            let route = Route::ranked(rank, Get, route.to_string(), dummy_handler);
            router.add(route);
        }

        router
    }

    fn router_with_unranked_routes(routes: &[&'static str]) -> Router {
        let mut router = Router::new();
        for route in routes {
            let route = Route::ranked(0, Get, route.to_string(), dummy_handler);
            router.add(route);
        }

        router
    }

    fn unranked_route_collisions(routes: &[&'static str]) -> bool {
        let router = router_with_unranked_routes(routes);
        router.has_collisions()
    }

    fn default_rank_route_collisions(routes: &[&'static str]) -> bool {
        let router = router_with_routes(routes);
        router.has_collisions()
    }

    #[test]
    fn test_collisions() {
        assert!(unranked_route_collisions(&["/hello", "/hello"]));
        assert!(unranked_route_collisions(&["/<a>", "/hello"]));
        assert!(unranked_route_collisions(&["/<a>", "/<b>"]));
        assert!(unranked_route_collisions(&["/hello/bob", "/hello/<b>"]));
        assert!(unranked_route_collisions(&["/a/b/<c>/d", "/<a>/<b>/c/d"]));
        assert!(unranked_route_collisions(&["/a/b", "/<a..>"]));
        assert!(unranked_route_collisions(&["/a/b/c", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/<a>/b", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/<b>", "/a/<a..>"]));
        assert!(unranked_route_collisions(&["/a/b/<c>", "/a/<a..>"]));
    }

    #[test]
    fn test_no_collisions() {
        assert!(!unranked_route_collisions(&["/<a>", "/a/<a..>"]));
        assert!(!unranked_route_collisions(&["/a/b", "/a/b/c"]));
        assert!(!unranked_route_collisions(&["/a/b/c/d", "/a/b/c/<d>/e"]));
    }

    #[test]
    fn test_none_collisions_when_ranked() {
        assert!(!default_rank_route_collisions(&["/<a>", "/hello"]));
        assert!(!default_rank_route_collisions(&["/hello/bob", "/hello/<b>"]));
        assert!(!default_rank_route_collisions(&["/a/b/c/d", "/<a>/<b>/c/d"]));
        assert!(!default_rank_route_collisions(&["/hi", "/<hi>"]));
        assert!(!default_rank_route_collisions(&["/hi", "/<hi>"]));
        assert!(!default_rank_route_collisions(&["/a/b", "/a/b/<c..>"]));
    }

    fn route<'a>(router: &'a Router,
                 method: Method,
                 uri: &str)
                 -> Option<&'a Route> {
        let request = Request::new(method, URI::new(uri));
        let matches = router.route(&request);
        if matches.len() > 0 {
            Some(matches[0])
        } else {
            None
        }
    }

    fn matches<'a>(router: &'a Router, method: Method, uri: &str) -> Vec<&'a Route> {
        let request = Request::new(method, URI::new(uri));
        router.route(&request)
    }

    #[test]
    fn test_ok_routing() {
        let router = router_with_routes(&["/hello"]);
        assert!(route(&router, Get, "/hello").is_some());

        let router = router_with_routes(&["/<a>"]);
        assert!(route(&router, Get, "/hello").is_some());
        assert!(route(&router, Get, "/hi").is_some());
        assert!(route(&router, Get, "/bobbbbbbbbbby").is_some());
        assert!(route(&router, Get, "/dsfhjasdf").is_some());

        let router = router_with_routes(&["/<a>/<b>"]);
        assert!(route(&router, Get, "/hello/hi").is_some());
        assert!(route(&router, Get, "/a/b/").is_some());
        assert!(route(&router, Get, "/i/a").is_some());
        assert!(route(&router, Get, "/jdlk/asdij").is_some());

        let mut router = Router::new();
        router.add(Route::new(Put, "/hello".to_string(), dummy_handler));
        router.add(Route::new(Post, "/hello".to_string(), dummy_handler));
        router.add(Route::new(Delete, "/hello".to_string(), dummy_handler));
        assert!(route(&router, Put, "/hello").is_some());
        assert!(route(&router, Post, "/hello").is_some());
        assert!(route(&router, Delete, "/hello").is_some());

        let router = router_with_routes(&["/<a..>"]);
        assert!(route(&router, Get, "/hello/hi").is_some());
        assert!(route(&router, Get, "/a/b/").is_some());
        assert!(route(&router, Get, "/i/a").is_some());
        assert!(route(&router, Get, "/a/b/c/d/e/f").is_some());

    }

    #[test]
    fn test_err_routing() {
        let router = router_with_routes(&["/hello"]);
        assert!(route(&router, Put, "/hello").is_none());
        assert!(route(&router, Post, "/hello").is_none());
        assert!(route(&router, Options, "/hello").is_none());
        assert!(route(&router, Get, "/hell").is_none());
        assert!(route(&router, Get, "/hi").is_none());
        assert!(route(&router, Get, "/hello/there").is_none());
        assert!(route(&router, Get, "/hello/i").is_none());
        assert!(route(&router, Get, "/hillo").is_none());

        let router = router_with_routes(&["/<a>"]);
        assert!(route(&router, Put, "/hello").is_none());
        assert!(route(&router, Post, "/hello").is_none());
        assert!(route(&router, Options, "/hello").is_none());
        assert!(route(&router, Get, "/hello/there").is_none());
        assert!(route(&router, Get, "/hello/i").is_none());

        let router = router_with_routes(&["/<a>/<b>"]);
        assert!(route(&router, Get, "/a/b/c").is_none());
        assert!(route(&router, Get, "/a").is_none());
        assert!(route(&router, Get, "/a/").is_none());
        assert!(route(&router, Get, "/a/b/c/d").is_none());
        assert!(route(&router, Put, "/hello/hi").is_none());
        assert!(route(&router, Put, "/a/b").is_none());
        assert!(route(&router, Put, "/a/b").is_none());
    }

    macro_rules! assert_ranked_routes {
        ($routes:expr, $to:expr, $want:expr) => ({
            let router = router_with_routes($routes);
            let route_path = route(&router, Get, $to).unwrap().path.as_str();
            assert_eq!(route_path as &str, $want as &str);
        })
    }

    #[test]
    fn test_default_ranking() {
        assert_ranked_routes!(&["/hello", "/<name>"], "/hello", "/hello");
        assert_ranked_routes!(&["/<name>", "/hello"], "/hello", "/hello");
        assert_ranked_routes!(&["/<a>", "/hi", "/<b>"], "/hi", "/hi");
        assert_ranked_routes!(&["/<a>/b", "/hi/c"], "/hi/c", "/hi/c");
        assert_ranked_routes!(&["/<a>/<b>", "/hi/a"], "/hi/c", "/<a>/<b>");
        assert_ranked_routes!(&["/hi/a", "/hi/<c>"], "/hi/c", "/hi/<c>");
    }

    fn ranked_collisions(routes: &[(isize, &'static str)]) -> bool {
        let router = router_with_ranked_routes(routes);
        router.has_collisions()
    }

    #[test]
    fn test_no_manual_ranked_collisions() {
        assert!(!ranked_collisions(&[(1, "a/<b>"), (2, "a/<b>")]));
        assert!(!ranked_collisions(&[(0, "a/<b>"), (2, "a/<b>")]));
        assert!(!ranked_collisions(&[(5, "a/<b>"), (2, "a/<b>")]));
        assert!(!ranked_collisions(&[(1, "a/<b>"), (1, "b/<b>")]));
    }

    macro_rules! assert_ranked_routing {
        (to: $to:expr, with: $routes:expr, expect: $($want:expr),+) => ({
            let router = router_with_ranked_routes(&$routes);
            let routed_to = matches(&router, Get, $to);
            let expected = &[$($want),+];
            assert!(routed_to.len() == expected.len());
            for (got, expected) in routed_to.iter().zip(expected.iter()) {
                assert_eq!(got.path.as_str() as &str, expected.1);
                assert_eq!(got.rank, expected.0);
            }
        })
    }

    #[test]
    fn test_ranked_routing() {
        assert_ranked_routing!(
            to: "a/b",
            with: [(1, "a/<b>"), (2, "a/<b>")],
            expect: (1, "a/<b>"), (2, "a/<b>")
        );

        assert_ranked_routing!(
            to: "b/b",
            with: [(1, "a/<b>"), (2, "b/<b>"), (3, "b/b")],
            expect: (2, "b/<b>"), (3, "b/b")
        );

        assert_ranked_routing!(
            to: "b/b",
            with: [(1, "a/<b>"), (2, "b/<b>"), (0, "b/b")],
            expect: (0, "b/b"), (2, "b/<b>")
        );

        assert_ranked_routing!(
            to: "/profile/sergio/edit",
            with: [(1, "/<a>/<b>/edit"), (2, "/profile/<d>"), (0, "/<a>/<b>/<c>")],
            expect: (0, "/<a>/<b>/<c>"), (1, "/<a>/<b>/edit")
        );

        assert_ranked_routing!(
            to: "/profile/sergio/edit",
            with: [(0, "/<a>/<b>/edit"), (2, "/profile/<d>"), (5, "/<a>/<b>/<c>")],
            expect: (0, "/<a>/<b>/edit"), (5, "/<a>/<b>/<c>")
        );

        assert_ranked_routing!(
            to: "/a/b",
            with: [(0, "/a/b"), (1, "/a/<b..>")],
            expect: (0, "/a/b"), (1, "/a/<b..>")
        );

        assert_ranked_routing!(
            to: "/a/b/c/d/e/f",
            with: [(1, "/a/<b..>"), (2, "/a/b/<c..>")],
            expect: (1, "/a/<b..>"), (2, "/a/b/<c..>")
        );
    }

    fn match_params(router: &Router, path: &str, expected: &[&str]) -> bool {
        println!("Testing: {} (expect: {:?})", path, expected);
        route(router, Get, path).map_or(false, |route| {
            let params = route.get_param_indexes(&URI::new(path));
            if params.len() != expected.len() {
                return false;
            }

            for (k, (i, j)) in params.into_iter().enumerate() {
                if &path[i..j] != expected[k] {
                    return false;
                }
            }

            true
        })
    }

    #[test]
    fn test_params() {
        let router = router_with_routes(&["/<a>"]);
        assert!(match_params(&router, "/hello", &["hello"]));
        assert!(match_params(&router, "/hi", &["hi"]));
        assert!(match_params(&router, "/bob", &["bob"]));
        assert!(match_params(&router, "/i", &["i"]));

        let router = router_with_routes(&["/hello"]);
        assert!(match_params(&router, "/hello", &[]));

        let router = router_with_routes(&["/<a>/<b>"]);
        assert!(match_params(&router, "/a/b", &["a", "b"]));
        assert!(match_params(&router, "/912/sas", &["912", "sas"]));

        let router = router_with_routes(&["/hello/<b>"]);
        assert!(match_params(&router, "/hello/b", &["b"]));
        assert!(match_params(&router, "/hello/sergio", &["sergio"]));

        let router = router_with_routes(&["/hello/<b>/age"]);
        assert!(match_params(&router, "/hello/sergio/age", &["sergio"]));
        assert!(match_params(&router, "/hello/you/age", &["you"]));
    }
}
