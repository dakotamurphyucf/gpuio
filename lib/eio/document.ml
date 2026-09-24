open Core
module Source = Gpuio.Text_source
module Registration = Document_registry.Registration
module Error = Gpuio_protocol.Document_wire.Error

type t =
  { registration : Registration.t
  ; mutable decoder : Source.Decoder.t
  }

let create app ~scope source =
  Bonsai.Effect.map
    (App.Expert.register_document app ~scope source)
    ~f:
      (Result.map ~f:(fun registration ->
         { registration; decoder = Source.Decoder.empty }))
;;

let handle t = Registration.handle t.registration
let source t = Registration.source t.registration
let is_published t = Registration.is_published t.registration
let error t = Registration.error t.registration

let release t =
  Registration.release t.registration;
  t.decoder <- Source.Decoder.empty
;;

let pending_bytes t =
  ignore (source t : Source.t option);
  Source.Decoder.pending_bytes t.decoder
;;

let change t f =
  let open Or_error.Let_syntax in
  let%bind source =
    Option.value_map
      (source t)
      ~default:(Or_error.error_string "document released")
      ~f:Or_error.return
  in
  let%bind next = f source in
  Result.map_error (Registration.set t.registration next) ~f:(fun error ->
    Core.Error.create_s [%sexp (error : Error.t)])
;;

let append t text =
  let%bind.Or_error () = Source.Decoder.finish t.decoder in
  change t (fun source -> Source.append source text)
;;

let push_bytes t bytes =
  let open Or_error.Let_syntax in
  let%bind decoder, ready = Source.Decoder.feed t.decoder bytes in
  let%map () = change t (fun source -> Source.append source ready) in
  t.decoder <- decoder
;;

let finish t =
  let%bind.Or_error () = Source.Decoder.finish t.decoder in
  change t Source.finish
;;

let cancel t =
  let%map.Or_error () = change t Source.cancel in
  t.decoder <- Source.Decoder.empty
;;

let reset t text =
  let%map.Or_error () = change t (fun source -> Source.reset source text) in
  t.decoder <- Source.Decoder.empty
;;

let edit t ~first ~last ~text =
  let%bind.Or_error () = Source.Decoder.finish t.decoder in
  change t (fun source -> Source.edit source ~first ~last ~text)
;;
