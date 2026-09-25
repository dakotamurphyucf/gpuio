open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module Asset = Gpuio_eio.Asset

module Name = struct
  type t =
    | Spark
    | Search
    | Message
    | Plus
    | Arrow_up
    | Paperclip
    | External
    | Sun
    | Moon
    | Command
    | Close
    | Code
    | Check
    | Stop
    | Retry
    | Sliders
    | History
    | Arrow_down
  [@@deriving equal]
end

type t = (Name.t * Gpuio.Asset.Handle.t) list B.Expert.Var.t

let create () = B.Expert.Var.create []
let value = B.Expert.Var.value

let paths =
  [ Name.Spark, "<path d='m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5Z'/>"
  ; Search, "<circle cx='10.5' cy='10.5' r='6.5'/><path d='m16 16 5 5'/>"
  ; ( Message
    , "<path d='M5 4h14a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H8l-5 3V6a2 2 0 0 1 2-2Z'/><path \
       d='M7 9h10M7 13h6'/>" )
  ; Plus, "<path d='M12 5v14M5 12h14'/>"
  ; Arrow_up, "<path d='M12 19V5m-6 6 6-6 6 6'/>"
  ; Paperclip, "<path d='m8 13 7-7a3 3 0 0 1 4 4l-9 9a5 5 0 0 1-7-7l9-9M6 15l9-9'/>"
  ; ( External
    , "<path d='M13 4h7v7m0-7-9 9M9 4H5a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h14a1 1 0 0 0 \
       1-1v-4'/>" )
  ; ( Sun
    , "<circle cx='12' cy='12' r='4'/><path d='M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 \
       1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5'/>" )
  ; Moon, "<path d='M20 15A9 9 0 0 1 9 4a9 9 0 1 0 11 11Z'/>"
  ; ( Command
    , "<path d='M8 8V5a2 2 0 1 0-2 2h11a2 2 0 1 0-2-2v13a2 2 0 1 0 2-2H6a2 2 0 1 0 2 \
       2V8Z'/>" )
  ; Close, "<path d='m6 6 12 12M6 18 18 6'/>"
  ; Code, "<path d='m8 6-6 6 6 6m8-12 6 6-6 6m-3-14-2 16'/>"
  ; Check, "<path d='m4 12 5 5L20 6'/>"
  ; Stop, "<rect x='6' y='6' width='12' height='12' rx='2'/>"
  ; Retry, "<path d='M4 11a8 8 0 1 1 2 6M4 4v7h7'/>"
  ; ( Sliders
    , "<path d='M4 7h5m5 0h6M4 17h10m5 0h1'/><circle cx='11.5' cy='7' r='2.5'/><circle \
       cx='16.5' cy='17' r='2.5'/>" )
  ; History, "<path d='M4 10a8 8 0 1 1 1 7M4 4v6h6m2-3v5l3 2'/>"
  ; Arrow_down, "<path d='M12 5v14m-6-6 6 6 6-6'/>"
  ]
;;

let initialize t app =
  let rec loop assets = function
    | [] -> E.of_thunk (fun () -> B.Expert.Var.set t (List.rev assets))
    | (name, path) :: rest ->
      let source =
        Asset.Source.of_bytes
          ~format:Svg
          ("<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 \
            24 24' fill='none' stroke='white' stroke-width='1.7' stroke-linecap='round' \
            stroke-linejoin='round'>"
           ^ path
           ^ "</svg>")
        |> Or_error.ok_exn
      in
      let open E.Let_syntax in
      let%bind result = Asset.register app ~scope:(Gpuio_eio.App.scope app) source in
      (match result with
       | Ok asset -> loop ((name, Asset.handle asset) :: assets) rest
       | Error error -> E.of_thunk (fun () -> raise_s [%sexp (error : Asset.Error.t)]))
  in
  loop [] paths
;;

let find assets name = List.Assoc.find assets name ~equal:Name.equal

let decoration assets name =
  Option.map (find assets name) ~f:(fun asset ->
    Gpuio.Icon.Decoration.create ~asset () |> Or_error.ok_exn)
;;

let view assets name =
  match find assets name with
  | None ->
    Gpuio_bonsai.View.column
      ~style:
        (Gpuio.Style.create_exn
           [ Width (Gpuio.Length.px_exn 16.); Height (Gpuio.Length.px_exn 16.) ])
      []
  | Some asset ->
    Gpuio_bonsai.View.icon
      ~style:
        (Gpuio.Style.create_exn
           [ Width (Gpuio.Length.px_exn 16.)
           ; Height (Gpuio.Length.px_exn 16.)
           ; Shrink 0.
           ])
      (Gpuio.Icon.Config.create ~asset ~description:Gpuio.Image.Description.decorative ()
       |> Or_error.ok_exn)
;;
