open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module N = Gpuio.Navigation_stack
module Stage = Run_diagram.Stage

module Route = struct
  type t =
    | Overview
    | Diagram
    | Review
    | Sources
    | Results
    | Stage of Stage.t
  [@@deriving equal]

  let label = function
    | Overview -> "Workspace"
    | Diagram -> "Run"
    | Review -> "Review"
    | Sources -> "Sources"
    | Results -> "Results"
    | Stage stage -> Stage.name stage
  ;;
end

type t =
  { opened : bool B.Expert.Var.t
  ; routes : Route.t N.t B.Expert.Var.t
  ; diagram : Diagram.t
  ; mutable serial : int
  }

let entry serial route =
  N.Entry.create
    ~id:(N.Id.of_string (Int.to_string serial) |> Or_error.ok_exn)
    ~label:(Route.label route)
    route
  |> Or_error.ok_exn
;;

let create () =
  { opened = B.Expert.Var.create false
  ; routes =
      B.Expert.Var.create
        (N.create [ entry 0 Overview; entry 1 Review ] |> Or_error.ok_exn)
  ; diagram = Diagram.create ()
  ; serial = 1
  }
;;

let toggle t = B.Expert.Var.set t.opened (not (B.Expert.Var.get t.opened))

let change t f =
  E.of_thunk (fun () -> B.Expert.Var.set t.routes (f (B.Expert.Var.get t.routes)))
;;

let navigate t ?(replace = false) route =
  change t (fun routes ->
    if
      Option.exists (N.current routes) ~f:(fun entry ->
        Route.equal (N.Entry.data entry) route)
    then routes
    else (
      t.serial <- t.serial + 1;
      (* Keep repeated exploration bounded without rejecting ordinary navigation.
       All page state is window-owned, independent of this visit history. *)
      let routes =
        if replace || List.length (N.entries routes) < 32
        then routes
        else N.singleton (entry 0 Overview)
      in
      (if replace then N.replace else N.push) routes (entry t.serial route)
      |> Or_error.ok_exn))
;;

let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let component t ~app ~window ~sources ~results ~dark graph =
  let review = Review.component ~dark graph in
  let active =
    B.map2
      (B.Expert.Var.value t.opened)
      (B.Expert.Var.value t.routes)
      ~f:(fun opened routes ->
        opened
        && Option.exists (N.current routes) ~f:(fun entry ->
          match N.Entry.data entry with
          | Diagram -> true
          | Overview | Review | Sources | Results | Stage _ -> false))
  in
  let diagram =
    Diagram.component
      t.diagram
      ~app
      ~window
      ~active
      ~dark
      ~on_open:(fun stage -> navigate t (Stage stage))
      graph
  in
  let sources_active =
    B.map2
      (B.Expert.Var.value t.opened)
      (B.Expert.Var.value t.routes)
      ~f:(fun opened routes ->
        opened
        && Option.exists (N.current routes) ~f:(fun entry ->
          match N.Entry.data entry with
          | Sources -> true
          | Overview | Diagram | Review | Results | Stage _ -> false))
  in
  let sources = Sources.component sources ~active:sources_active ~dark graph in
  let results_active =
    B.map2
      (B.Expert.Var.value t.opened)
      (B.Expert.Var.value t.routes)
      ~f:(fun opened routes ->
        opened
        && Option.exists (N.current routes) ~f:(fun entry ->
          match N.Entry.data entry with
          | Results -> true
          | Overview | Diagram | Review | Sources | Stage _ -> false))
  in
  let results = Results.component results ~active:results_active ~dark graph in
  let open B.Let_syntax in
  let%arr opened = B.Expert.Var.value t.opened
  and routes = B.Expert.Var.value t.routes
  and dark = dark
  and diagram = diagram
  and sources = sources
  and results = results
  and review = review in
  let palette = Palette.of_dark dark in
  let button ?(disabled = false) label on_click =
    V.button
      ~disabled
      ~on_click
      label
      ~style:
        (style
           [ Foreground palette.text
           ; Background (Gpuio.Background.solid palette.raised)
           ; Border_color palette.line
           ; Border_width 1.
           ; Padding (px 8.)
           ; Radius 8.
           ])
  in
  let crumbs =
    let path = N.back_entries routes @ Option.to_list (N.current routes) in
    let path =
      if List.length path <= 3
      then List.map path ~f:Option.some
      else
        [ Some (List.hd_exn path); None ]
        @ List.map (List.drop path (List.length path - 2)) ~f:Option.some
    in
    path
    |> List.map ~f:(function
      | None ->
        Gpuio.Choice.create
          ~id:(Gpuio.Choice.Id.of_string "omitted" |> Or_error.ok_exn)
          ~label:"…"
          ~disabled:true
          ()
        |> Or_error.ok_exn
      | Some entry ->
        Gpuio.Choice.create
          ~id:
            (Gpuio.Choice.Id.of_string (N.Id.to_string (N.Entry.id entry))
             |> Or_error.ok_exn)
          ~label:(N.Entry.label entry)
          ()
        |> Or_error.ok_exn)
    |> Gpuio.Choice.Collection.create
    |> Or_error.ok_exn
  in
  let breadcrumb =
    Gpuio.Navigation.breadcrumbs
      crumbs
      ~label:"Artifact path"
      ~current_description:"Current artifact"
      ~appearance:
        (Gpuio.Navigation.Appearance.create
           ~item_style:
             (style
                [ Foreground palette.muted
                ; Font_size 12.
                ; Background (Gpuio.Background.solid palette.sidebar)
                ; Border_width 0.
                ; Border_color palette.line
                ; Padding (px 4.)
                ])
           ~current_style:
             (style
                [ Foreground palette.accent
                ; Font_size 12.
                ; Background (Gpuio.Background.solid palette.accent_surface)
                ; Border_bottom_width 0.
                ])
           ~separator_style:(style [ Border_color palette.line ])
           ())
      ~on_navigate:(fun id ->
        change t (fun routes ->
          match
            N.pop_to
              routes
              (N.Id.of_string (Gpuio.Choice.Id.to_string id) |> Or_error.ok_exn)
          with
          | Ok routes -> routes
          | Error _ -> routes))
      ()
    |> Or_error.ok_exn
  in
  let page entry =
    match N.Entry.data entry with
    | Route.Review -> [ review ]
    | Diagram -> [ diagram ]
    | Sources -> [ sources ]
    | Results -> [ results ]
    | Overview ->
      [ V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) "A closer look."
      ; V.text
          ~style:(style [ Foreground palette.muted; Line_height (px 20.) ])
          "Explore a simulated run, inspect its stages, and keep your review progress \
           beside the conversation."
      ; button "Explore run diagram" (navigate t Diagram)
      ; button "Explore sources" (navigate t Sources)
      ; button "Explore results" (navigate t Results)
      ; button "Review checkpoints" (navigate t Review)
      ]
    | Stage stage ->
      [ V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) (Stage.name stage)
      ; V.text
          ~style:(style [ Foreground palette.muted; Line_height (px 20.) ])
          (Stage.description stage)
      ; V.text
          ~style:(style [ Font_size 12.; Foreground palette.accent ])
          "SIMULATED · NO FILES ARE MODIFIED"
      ; button "Next stage" (navigate t ~replace:true (Stage (Stage.next stage)))
      ; button "Review checkpoints" (navigate t Review)
      ]
  in
  V.panel
    ~key:(Gpuio.Key.of_string_exn "artifact-inspector")
    ~label:"Artifact workspace"
    ~active:opened
    ~hidden:Unmount
    ~style:
      (style
         [ Width (px 380.)
         ; Height (Gpuio.Length.percent_exn 100.)
         ; Shrink 0.
         ; Min_height (px 0.)
         ; Background (Gpuio.Background.solid palette.sidebar)
         ; Foreground palette.text
         ; Border_left_width 1.
         ; Border_color palette.line
         ])
    [ V.row
        ~style:
          (style
             [ Height (px 45.)
             ; Shrink 0.
             ; Align_items Center
             ; Justify_content Space_between
             ; Padding_left (px 20.)
             ; Padding_right (px 12.)
             ; Border_bottom_width 1.
             ; Border_color palette.line
             ])
        [ V.text ~style:(style [ Font_size 12.; Font_weight 600 ]) "ARTIFACT WORKSPACE"
        ; V.button
            ~accessible_name:"Close workspace inspector"
            ~style:
              (style
                 [ Foreground palette.text
                 ; Background (Gpuio.Background.solid palette.raised)
                 ; Border_color palette.line
                 ; Border_width 1.
                 ; Padding (px 8.)
                 ; Radius 8.
                 ])
            ~on_click:(E.of_thunk (fun () -> B.Expert.Var.set t.opened false))
            "Close"
        ]
    ; V.column
        ~style:(style [ Padding (px 14.); Gap (px 10.); Shrink 0. ])
        [ V.row
            ~style:(style [ Gap (px 6.) ])
            [ button ~disabled:(not (N.can_pop routes)) "Back" (change t N.pop)
            ; button ~disabled:(not (N.can_forward routes)) "Forward" (change t N.forward)
            ; button "Workspace" (change t N.pop_to_root)
            ; button "Diagram" (navigate t Diagram)
            ]
        ; breadcrumb
        ]
    ; V.navigation_stack
        routes
        ~label:"Artifact pages"
        ~hidden:Unmount
        ~style:
          (style [ Grow 1.; Min_height (px 0.); Width (Gpuio.Length.percent_exn 100.) ])
        ~page_style:
          (style
             [ Height (Gpuio.Length.percent_exn 100.)
             ; Width (Gpuio.Length.percent_exn 100.)
             ; Padding (px 22.)
             ; Gap (px 16.)
             ; Overflow_y Scroll
             ])
        ~content:page
        ()
    ]
;;
