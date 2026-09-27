open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Asset = Gpuio_eio.Asset
module App = Gpuio_eio.App

module Resources = struct
  type t =
    | Absent
    | Pending
    | Ready of
        { portrait : Gpuio.Asset.Handle.t
        ; unavailable : Gpuio.Asset.Handle.t
        }
    | Failed
end

module Mode = struct
  type t =
    | Initials
    | Portrait
    | Unavailable
  [@@deriving equal]
end

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let initialize resources ~app ~window =
  let open E.Let_syntax in
  let%bind start =
    E.of_thunk (fun () ->
      match B.Expert.Var.get resources with
      | Resources.Pending | Ready _ -> false
      | Absent | Failed ->
        B.Expert.Var.set resources Pending;
        true)
  in
  if not start
  then E.Ignore
  else (
    let scope = App.Window.scope window in
    let source =
      Asset.Source.of_bytes
        ~format:Svg
        {|<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" rx="32" fill="#7560C4"/><path d="M32 12L37 27L52 32L37 37L32 52L27 37L12 32L27 27Z" fill="#F3EEFF"/></svg>|}
      |> ok
    in
    let%bind portrait = Asset.register app ~scope source in
    match portrait with
    | Error _ -> E.of_thunk (fun () -> B.Expert.Var.set resources Failed)
    | Ok portrait ->
      let source =
        Asset.Source.of_bytes ~format:Pnm "Deliberately unavailable local portrait" |> ok
      in
      let%bind unavailable = Asset.register app ~scope source in
      E.of_thunk (fun () ->
        match unavailable with
        | Error _ ->
          Asset.release portrait;
          B.Expert.Var.set resources Failed
        | Ok unavailable ->
          B.Expert.Var.set
            resources
            (Ready
               { portrait = Asset.handle portrait
               ; unavailable = Asset.handle unavailable
               })))
;;

let component ~app ~window ~dark graph =
  let resources = B.Expert.Var.create Resources.Absent in
  let mode, set_mode = B.state Mode.Initials graph in
  let observed, set_observed = B.state Gpuio.Image.State.Loading graph in
  let open B.Let_syntax in
  let%arr resources_value = B.Expert.Var.value resources
  and mode = mode
  and set_mode = set_mode
  and observed = observed
  and set_observed = set_observed
  and dark = dark in
  let p = Palette.of_dark dark in
  let asset =
    match resources_value, mode with
    | Ready { portrait; _ }, Mode.Portrait -> Some portrait
    | Ready { unavailable; _ }, Unavailable -> Some unavailable
    | (Absent | Pending | Failed | Ready _), _ -> None
  in
  let button label selected =
    V.button
      ~disabled:
        (Mode.equal selected mode
         &&
         match resources_value with
         | Failed -> false
         | Absent | Pending | Ready _ -> true)
      ~style:
        (style
           [ Padding (px 7.)
           ; Radius 7.
           ; Foreground p.text
           ; Background (Gpuio.Background.solid p.raised)
           ; Border_color p.line
           ; Border_width 1.
           ; Font_size 12.
           ])
      ~on_click:
        (E.Many
           [ set_mode selected; set_observed Loading; initialize resources ~app ~window ])
      label
  in
  let status =
    match mode, resources_value with
    | Mode.Initials, _ -> "Contributor initials"
    | _, (Absent | Pending) -> "Loading local portrait…"
    | _, Failed -> "Portrait registration failed. Choose a fixture to retry."
    | _, Ready _ ->
      (match observed with
       | Loading -> "Decoding local portrait…"
       | Ready _ -> "Local portrait ready"
       | Failed _ -> "Portrait unavailable · showing initials")
  in
  V.column
    ~style:(style [ Gap (px 10.) ])
    [ V.row
        ~style:(style [ Gap (px 10.); Align_items Center ])
        [ V.avatar
            ~on_change:set_observed
            ~style:
              (style
                 [ Background (Gpuio.Background.solid p.accent_surface)
                 ; Foreground p.accent
                 ; Width (px 36.)
                 ; Height (px 36.)
                 ; Font_size 13.
                 ])
            (Gpuio.Avatar.Config.create
               ?asset
               ~fallback:(Gpuio.Avatar.Fallback.create "GP" |> ok)
               ~description:(Gpuio.Image.Description.label "GPUIO local assistant" |> ok)
               ())
        ; V.text ~style:(style [ Font_weight 600 ]) "GPUIO · Local assistant"
        ]
    ; V.text ~style:(style [ Foreground p.muted; Font_size 12. ]) status
    ; V.row
        ~style:(style [ Gap (px 6.); Wrap Wrap ])
        [ button "Local portrait" Portrait; button "Unavailable portrait" Unavailable ]
    ]
;;
