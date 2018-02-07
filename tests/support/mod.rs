pub extern crate futures;
pub extern crate hyper;
pub extern crate tokio_core;

pub use std::net::SocketAddr;
pub use self::futures::{Future, Stream};
pub use self::hyper::Method::*;
pub use self::hyper::StatusCode;

macro_rules! t {
    (
        $name:ident,
        client: $(
            request: $(
                $c_req_prop:ident: $c_req_val:expr,
            )*;
            response: $(
                $c_res_prop:ident: $c_res_val:expr,
            )*;
        )*
        server: $(
            request: $(
                $s_req_prop:ident: $s_req_val:expr,
            )*;
            response: $(
                $s_res_prop:ident: $s_res_val:expr,
            )*
        )*
    ) => (
        #[test]
        fn $name() {
            let c = vec![$((
                __CReq {
                    $($c_req_prop: From::from($c_req_val),)*
                    ..Default::default()
                },
                __CRes {
                    $($c_res_prop: From::from($c_res_val),)*
                    ..Default::default()
                }
            ),)*];
            let s = vec![$((
                __SReq {
                    $($s_req_prop: From::from($s_req_val),)*
                    ..Default::default()
                },
                __SRes {
                    $($s_res_prop: From::from($s_res_val),)*
                    ..Default::default()
                }
            ),)*];
            __run_test(__TestConfig {
                client_version: 1,
                client_msgs: c.clone(),
                server_version: 1,
                server_msgs: s.clone(),
            });

            #[cfg(feature = "http2")]
            __run_test(__TestConfig {
                client_version: 2,
                client_msgs: c,
                server_version: 2,
                server_msgs: s,
            });
        }
    );
}

#[derive(Clone, Debug, Default)]
pub struct __CReq {
    pub method: hyper::Method,
    pub uri: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct __CRes {
    pub status: hyper::StatusCode,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct __SReq {
    pub method: hyper::Method,
    pub uri: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct __SRes {
    pub status: hyper::StatusCode,
    pub body: Vec<u8>,
}

pub struct __TestConfig {
    pub client_version: usize,
    pub client_msgs: Vec<(__CReq, __CRes)>,

    pub server_version: usize,
    pub server_msgs: Vec<(__SReq, __SRes)>,
}

pub fn __run_test(cfg: __TestConfig) {
    extern crate pretty_env_logger;
    use hyper::{Body, Client, Request, Response};
    let _ = pretty_env_logger::try_init();
    let mut core = tokio_core::reactor::Core::new().expect("new core");
    let handle = core.handle();

    #[allow(unused_mut)]
    let mut config = Client::configure();
    #[cfg(feature = "http2")]
    {
        if cfg.client_version == 2 {
            config = config.http2_only();
        }
    }
    let client = config.build(&handle);

    let serve_handles = ::std::sync::Mutex::new(
        cfg.server_msgs.into_iter()
            .map(|(_sreq, sres)| {

                Response::new()
                    .with_status(sres.status)
                    .with_body(sres.body)
            })
            .collect::<Vec<_>>()
    );
    let service = hyper::server::service_fn(move |_req: Request<Body>| -> Result<Response<Body>, hyper::Error> {
        let res = serve_handles.lock()
            .unwrap()
            .remove(0);
        Ok(res)
    });
    let new_service = hyper::server::const_service(service);

    #[allow(unused_mut)]
    let mut http = hyper::server::Http::new();
    #[cfg(feature = "http2")]
    {
        if cfg.server_version == 2 {
            http.http2();
        }
    }
    let serve = http.serve_addr_handle2(
            &SocketAddr::from(([127, 0, 0, 1], 0)),
            &handle,
            new_service,
        )
        .expect("serve_addr_handle");

    let addr = serve.incoming_ref().local_addr();
    let handle2 = handle.clone();
    handle.spawn(serve.for_each(move |conn: hyper::server::Connection2<_, _>| {
        handle2.spawn(conn.map(|_| ()).map_err(|e| panic!("server connection error: {}", e)));
        Ok(())
    }).map_err(|e| panic!("serve error: {}", e)));

    for (creq, cres) in cfg.client_msgs {
        let uri = format!("http://{}{}", addr, creq.uri).parse().expect("uri parse");
        let req = Request::new(creq.method, uri);
        let cstatus = cres.status;
        let cbody = cres.body;
        let fut = client.request(req)
            .and_then(move |res| {
                assert_eq!(res.status(), cstatus);
                //assert_eq!(res.version(), c_version);
                res.body().concat2()
            })
            .and_then(move |body| {
                assert_eq!(body.as_ref(), cbody.as_slice());
                Ok(())
            });
        core.run(fut).unwrap();
    }
}
