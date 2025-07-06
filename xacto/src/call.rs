#[macro_export]
macro_rules! call {
    ($act:expr, $msg:path) => {{
        async {
            $act.call(|reply| $msg( reply )).await
        }
    }};
    ($act:expr, $msg:path, $( $arg:expr ),+ $(,)? ) => {{
        async {
            $act.call(|reply| $msg( $( $arg ),*, reply )).await
        }
    }};
}

#[macro_export]
macro_rules! try_call {
    ($act:expr, $msg:path) => {{
        async {
            $act.try_call(|reply| $msg( reply )).await
        }
    }};
    ($act:expr, $msg:path, $( $arg:expr ),+ $(,)? ) => {{
        async {
            $act.try_call(|reply| $msg( $( $arg ),*, reply )).await
        }
    }};
}
