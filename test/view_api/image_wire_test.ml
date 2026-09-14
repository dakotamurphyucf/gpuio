open Core
module W = Gpuio_protocol.Wire
module I = W.Image

let%expect_test
    "image wire layout agrees with independent fixtures; invalid metadata rejects"
  =
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler =
    Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let asset =
    Gpuio_protocol.Resource_id.create ~slot:2L ~generation:3L |> Or_error.ok_exn
  in
  let errors =
    [ I.Error.Wrong_application
    ; Released
    ; Invalid_data
    ; Unsupported
    ; Resource_limit
    ; Native_failure
    ]
  in
  let configs =
    List.map [ I.Fit.Fill; Contain; Cover; Scale_down; None ] ~f:(fun fit ->
      { I.Config.source = Reference asset; fit; label = Some "Preview" })
    @ List.map errors ~f:(fun error ->
      { I.Config.source = Unavailable error; fit = Contain; label = None })
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ W.Op.Create (node, Image, "", Some handler) ]
          @ List.map configs ~f:(fun config -> W.Op.Set_image (node, config))
          @ [ W.Op.Set_root (Some node) ]
      }
  in
  let states =
    [ I.State.Loading; Ready { width_px = 4L; height_px = 4L; frames = 1L } ]
    @ List.map errors ~f:(fun error -> I.State.Failed error)
  in
  let events =
    List.map states ~f:(fun state ->
      W.Event.Image_state (window, node, handler, 1L, state))
  in
  let encode events =
    Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events |> Bigstring.to_string
  in
  Eio_main.run (fun env ->
    let read name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(2 * i) ~len:2)))
    in
    assert (
      String.equal
        (read "images-v1-request.hex")
        (W.Message.encode message |> Or_error.ok_exn));
    let icon =
      match message with
      | Apply transaction ->
        W.Message.Apply
          { transaction with
            operations =
              List.map transaction.operations ~f:(function
                | W.Op.Create (node, Image, text, handler) ->
                  W.Op.Create (node, Icon, text, handler)
                | operation -> operation)
          }
      | _ -> assert false
    in
    assert (
      String.equal (read "icons-v1-request.hex") (W.Message.encode icon |> Or_error.ok_exn));
    let bytes = read "images-v1-events.hex" in
    assert (String.equal bytes (encode events));
    assert (List.equal W.Event.equal events (W.Event.decode bytes |> Or_error.ok_exn));
    for length = 0 to String.length bytes - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix bytes length)))
    done;
    assert (Result.is_error (W.Event.decode (bytes ^ "\000"))));
  List.iter
    [ -1L, 1L, 1L; 1L, 1L, 0L; 16384L, 16384L, 120L; Int64.max_value, 1L, 1L ]
    ~f:(fun (width_px, height_px, frames) ->
      let state = I.State.Ready { width_px; height_px; frames } in
      assert (Result.is_error (Gpuio.Image.Expert.state_of_wire state));
      assert (
        Result.is_error
          (W.Event.decode (encode [ Image_state (window, node, handler, 1L, state) ]))));
  [%expect {| |}]
;;
