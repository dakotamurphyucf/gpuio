(** Output through an explicitly supplied Eio sink.

    Blocking Unix character devices use an Eio system thread: the pinned macOS
    poll backend reports POLLNVAL for valid /dev/null descriptors. Other sinks
    retain normal Eio flow behavior. This does not alter descriptor flags, close
    the caller's sink, replace standard streams or patch the Eio runtime.

    Admitted descriptor inspection and blocking device writes finish before
    cancellation is delivered;
    a kernel write cannot safely be aborted by closing/reusing its descriptor.
    No synchronous native layout/input callback may call this function. *)
val write : _ Eio.Flow.sink -> string -> unit
