open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module A = Avatar_group
module Registered = Gpuio_eio.Asset

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn

(* Original geometric person icon, owned by this example. *)
let person_svg =
  {|<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="7" r="4"/><path d="M4 22v-3a8 8 0 0 1 16 0v3z"/></svg>|}
;;

let component app window palette graph =
  let resources =
    Preview_scope.acquire
      window
      ~name:"gallery team avatars"
      ~create:(fun scope ->
        let register ?(format = Asset.Format.Pnm) bytes =
          Bonsai.Effect.map
            (Registered.register app ~scope (Asset.Source.of_bytes ~format bytes |> ok))
            ~f:(fun result ->
              Result.map result ~f:Registered.handle
              |> Result.map_error ~f:(fun error ->
                Error.create_s [%sexp (error : Registered.Error.t)]))
        in
        Bonsai.Effect.bind (register Image_samples.gradient_pnm) ~f:(function
          | Error error -> Bonsai.Effect.return (Error error)
          | Ok image ->
            Bonsai.Effect.bind (register "Invalid avatar bytes") ~f:(function
              | Error error -> Bonsai.Effect.return (Error error)
              | Ok invalid ->
                Bonsai.Effect.map
                  (register ~format:Svg person_svg)
                  ~f:(Result.map ~f:(fun icon -> image, invalid, icon)))))
      graph
  in
  let limit, next_limit =
    B.state_machine0 ~default_model:3 ~apply_action:(fun _ n () -> (n + 1) % 6) graph
  in
  let size, next_size =
    B.state_machine0 ~default_model:2 ~apply_action:(fun _ n () -> (n + 1) % 5) graph
  in
  let overlap, next_overlap =
    B.state_machine0 ~default_model:1 ~apply_action:(fun _ n () -> (n + 1) % 3) graph
  in
  let source, next_source =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> (n + 1) % 3) graph
  in
  let reversed, toggle_reversed = B.toggle ~default_model:false graph in
  let show_overflow, toggle_overflow = B.toggle ~default_model:true graph in
  let actionable, toggle_actionable = B.toggle ~default_model:false graph in
  let identity_colors, toggle_identity_colors = B.toggle ~default_model:true graph in
  let rich, toggle_rich = B.toggle ~default_model:false graph in
  let squared, toggle_squared = B.toggle ~default_model:false graph in
  let actions, open_team =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let image_state, set_image_state = B.state Image.State.Loading graph in
  let open B.Let_syntax in
  let%arr p = palette
  and resources = resources
  and limit = limit
  and next_limit = next_limit
  and size = size
  and next_size = next_size
  and overlap = overlap
  and next_overlap = next_overlap
  and source = source
  and next_source = next_source
  and reversed = reversed
  and toggle_reversed = toggle_reversed
  and show_overflow = show_overflow
  and toggle_overflow = toggle_overflow
  and actionable = actionable
  and toggle_actionable = toggle_actionable
  and identity_colors = identity_colors
  and toggle_identity_colors = toggle_identity_colors
  and rich = rich
  and toggle_rich = toggle_rich
  and squared = squared
  and toggle_squared = toggle_squared
  and actions = actions
  and open_team = open_team
  and image_state = image_state
  and set_image_state = set_image_state in
  match resources with
  | Loading -> Palette.text p "Preparing team avatars…"
  | Failed error ->
    Palette.text p ("Team avatar resources unavailable: " ^ Error.to_string_hum error)
  | Ready (image, invalid, icon) ->
    let size_name, size =
      [| "XS", A.Size.xsmall
       ; "S", A.Size.small
       ; "M", A.Size.medium
       ; "L", A.Size.large
       ; "56", A.Size.of_pixels 56. |> ok
      |].(size)
    in
    let overlap = [| 0.; 0.3; 0.6 |].(overlap) in
    let source_name =
      [| (if rich then "Custom fallback" else "Initials"); "Image"; "Invalid image" |].(source)
    in
    let people =
      [ "ada", "AL", "Ada"
      ; "grace", "GH", "Grace"
      ; "yuki", "京都", "Yuki"
      ; "sam", "SC", "Sam"
      ; "morgan", "MR", "Morgan"
      ]
    in
    let items =
      List.map people ~f:(fun (id, initials, name) ->
        let asset =
          if String.equal id "ada"
          then (
            match source with
            | 0 -> None
            | 1 -> Some image
            | _ -> Some invalid)
          else None
        in
        let colors =
          if identity_colors && Option.is_none asset
          then
            Avatar.Palette.for_key (key id) ~appearance:(Palette.avatar_appearance p)
            |> Avatar.Palette.style
          else
            style
              [ Background (Background.solid (Palette.surface p))
              ; Foreground (Palette.foreground p)
              ; Border_color (Palette.border p)
              ]
        in
        let create =
          if rich
          then (
            let icon_size = A.Size.pixels size *. 0.5 in
            let fallback =
              V.icon
                ~style:(style [ Width (px icon_size); Height (px icon_size) ])
                (Icon.Config.create
                   ~asset:icon
                   ~description:Image.Description.decorative
                   ()
                 |> ok)
            in
            fun ~key ~style ?on_change config ->
              A.Item.create_with_fallback ~key ~style ?on_change config ~fallback |> ok)
          else
            fun ~key ~style ?on_change config ->
              A.Item.create ~key ~style ?on_change config
        in
        create
          ~key:(key id)
          ~style:
            (Style.merge
               [ colors
               ; style
                   ([ Style.Property.Border_width 2. ]
                    @ if squared then [ Radius 8. ] else [])
               ])
          ?on_change:(Option.some_if (String.equal id "ada") set_image_state)
          (Avatar.Config.create
             ?asset
             ~fallback:(Avatar.Fallback.create initials |> ok)
             ~description:(Image.Description.label ("Team member " ^ name) |> ok)
             ()))
    in
    let overflow ~omitted ~size =
      let label = sprintf "%d more teammates" omitted in
      if actionable
      then Palette.button p ("Show " ^ label) (open_team ())
      else
        A.ellipsis
          ~size
          ~style:(style [ Foreground (Palette.muted p) ])
          ~description:(Image.Description.label label |> ok)
          ()
    in
    let avatars =
      A.create
        ~size
        ~limit
        ~overlap
        ?overflow:(Option.some_if show_overflow overflow)
        (if reversed then List.rev items else items)
      |> ok
    in
    let avatars =
      V.with_accessibility
        avatars
        (Accessibility.create ~role:Group ~label:"Team avatars" () |> ok)
      |> ok
    in
    let checkbox label checked on_toggle =
      V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
    in
    let status =
      if source = 0
      then "No image source"
      else (
        match image_state with
        | Loading -> "Loading"
        | Ready _ -> "Ready"
        | Failed error -> "Failed: " ^ Sexp.to_string (Image.Error.sexp_of_t error))
    in
    V.column
      ~style:(style [ Gap (px 12.) ])
      [ Palette.text p ~muted:true "A familiar face. A team in a little space."
      ; avatars
      ; V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          [ Palette.button p (sprintf "Avatar limit: %d" limit) (next_limit ())
          ; Palette.button p ("Avatar size: " ^ size_name) (next_size ())
          ; Palette.button
              p
              (sprintf "Avatar overlap: %.0f%%" (100. *. overlap))
              (next_overlap ())
          ; Palette.button p ("Avatar source: " ^ source_name) (next_source ())
          ]
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ checkbox "Reverse avatar order" reversed toggle_reversed
          ; checkbox "Show avatar overflow" show_overflow toggle_overflow
          ; checkbox "Actionable avatar overflow" actionable toggle_actionable
          ; checkbox "Avatar identity colors" identity_colors toggle_identity_colors
          ; checkbox "Custom avatar fallback" rich toggle_rich
          ; checkbox "Rounded-square avatars" squared toggle_squared
          ]
      ; Palette.text
          p
          ~muted:true
          (sprintf "Avatar image: %s · Team opens: %d" status actions)
      ]
;;
