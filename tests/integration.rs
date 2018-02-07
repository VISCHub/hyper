#[macro_use]
mod support;
use self::support::*;

t! {
    get_1,
    client:
        request:
            uri: "/",
            ;
        response:
            status: StatusCode::Ok,
            ;
    server:
        request:
            uri: "/",
            ;
        response:
}

t! {
    get_body,
    client:
        request:
            uri: "/",
            ;
        response:
            status: StatusCode::Ok,
            body: "hello world",
            ;
    server:
        request:
            uri: "/",
            ;
        response:
            body: "hello world",
}
