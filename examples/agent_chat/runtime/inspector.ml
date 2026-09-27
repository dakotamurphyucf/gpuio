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
    | Feedback
    | Tour
    | Sources
    | Results
    | Stage of Stage.t
  [@@deriving equal]

  let label = function
    | Overview -> "Workspace"
    | Diagram -> "Run"
    | Review -> "Review"
    | Feedback -> "Feedback"
    | Tour -> "Tour"
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

let navigation t ~icons ~dark graph =
  let current =
    B.map (B.Expert.Var.value t.routes) ~f:(fun routes ->
      match Option.map (N.current routes) ~f:N.Entry.data with
      | None | Some Overview -> Artifact_sidebar.Destination.Overview
      | Some (Diagram | Stage _) -> Diagram
      | Some Review -> Review
      | Some Feedback -> Feedback
      | Some Tour -> Tour
      | Some Sources -> Sources
      | Some Results -> Results)
  in
  Artifact_sidebar.component
    ~icons
    ~current
    ~dark
    ~on_select:(fun destination ->
      let route =
        match destination with
        | Artifact_sidebar.Destination.Overview -> Route.Overview
        | Diagram -> Diagram
        | Review -> Review
        | Feedback -> Feedback
        | Tour -> Tour
        | Sources -> Sources
        | Results -> Results
      in
      E.Many [ E.of_thunk (fun () -> B.Expert.Var.set t.opened true); navigate t route ])
    graph
;;

let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn

let component t ~app ~window ~sources ~results ~annotation ~dark graph =
  let context_expanded, toggle_context =
    B.state_machine0
      ~default_model:false
      ~apply_action:(fun _ expanded () -> not expanded)
      graph
  in
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
          | Overview | Review | Feedback | Tour | Sources | Results | Stage _ -> false))
  in
  let diagram =
    Diagram.component
      t.diagram
      ~app
      ~window
      ~active
      ~dark
      ~annotation
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
          | Overview | Diagram | Review | Feedback | Tour | Results | Stage _ -> false))
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
          | Overview | Diagram | Review | Feedback | Tour | Sources | Stage _ -> false))
  in
  let results = Results.component results ~active:results_active ~dark graph in
  let feedback_active =
    B.map2
      (B.Expert.Var.value t.opened)
      (B.Expert.Var.value t.routes)
      ~f:(fun opened routes ->
        opened
        && Option.exists (N.current routes) ~f:(fun entry ->
          Route.equal (N.Entry.data entry) Feedback))
  in
  let feedback =
    Review_feedback.component
      ~app
      ~window
      ~active:feedback_active
      ~dark
      ~on_sources:(navigate t Sources)
      graph
  in
  let tour_active =
    B.map2
      (B.Expert.Var.value t.opened)
      (B.Expert.Var.value t.routes)
      ~f:(fun opened routes ->
        opened
        && Option.exists (N.current routes) ~f:(fun entry ->
          Route.equal (N.Entry.data entry) Tour))
  in
  let tour =
    Artifact_tour.component
      ~active:tour_active
      ~dark
      ~on_open:(fun page ->
        navigate
          t
          (match page with
           | Artifact_tour.Page.Sources -> Sources
           | Results -> Results
           | Diagram -> Diagram
           | Feedback -> Feedback))
      graph
  in
  let open B.Let_syntax in
  let%arr opened = B.Expert.Var.value t.opened
  and routes = B.Expert.Var.value t.routes
  and dark = dark
  and diagram = diagram
  and sources = sources
  and results = results
  and review = review
  and feedback = feedback
  and tour = tour
  and context_expanded = context_expanded
  and toggle_context = toggle_context in
  let palette = Palette.of_dark dark in
  let button ?(disabled = false) ?accessible_name label on_click =
    V.button
      ~disabled
      ?accessible_name
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
    | Route.Review -> [ review; button "Review feedback" (navigate t Feedback) ]
    | Feedback -> [ feedback ]
    | Tour -> [ tour ]
    | Diagram -> [ diagram ]
    | Sources -> [ sources ]
    | Results -> [ results ]
    | Overview ->
      [ V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) "A closer look."
      ; V.text
          ~style:(style [ Foreground palette.muted; Line_height (px 20.) ])
          "Explore a simulated run, inspect its stages, and keep your review progress \
           beside the conversation."
      ; Chat_motion.destination ~index:0 (button "Take workspace tour" (navigate t Tour))
      ; Chat_motion.destination
          ~index:1
          (button "Explore run diagram" (navigate t Diagram))
      ; Chat_motion.destination ~index:2 (button "Explore sources" (navigate t Sources))
      ; Chat_motion.destination ~index:3 (button "Explore results" (navigate t Results))
      ; Chat_motion.destination ~index:4 (button "Review checkpoints" (navigate t Review))
      ; Chat_motion.destination ~index:5 (button "Review feedback" (navigate t Feedback))
      ]
    | Stage stage ->
      [ V.text ~style:(style [ Font_size 23.; Font_weight 600 ]) (Stage.name stage)
      ; V.text
          ~style:(style [ Foreground palette.muted; Line_height (px 20.) ])
          (Stage.description stage)
      ; V.text
          ~style:(style [ Font_size 12.; Foreground palette.accent ])
          "SIMULATED · NO FILES ARE MODIFIED"
      ; button
          (if context_expanded then "Hide stage context" else "Show stage context")
          (toggle_context ())
      ; Chat_motion.stage_context
          ~expanded:context_expanded
          [ V.text ~style:(style [ Font_weight 600 ]) "Local run context"
          ; V.text "Execution: simulated on this device"
          ; V.text "Source files: unchanged"
          ; V.text ("Up next: " ^ Stage.name (Stage.next stage))
          ]
      ; button "Next stage" (navigate t ~replace:true (Stage (Stage.next stage)))
      ; button "Review checkpoints" (navigate t Review)
      ; button "Review feedback" (navigate t Feedback)
      ]
  in
  let heading ~compact =
    V.row
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
      [ V.text
          ~style:(style [ Font_size 12.; Font_weight 600 ])
          (if compact then "ARTIFACTS" else "ARTIFACT WORKSPACE")
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
  in
  let navigation ~compact =
    V.row
      ~style:
        (style
           [ Width (Gpuio.Length.percent_exn 100.)
           ; Height (px 36.)
           ; Gap (px 6.)
           ; Align_items Center
           ])
      [ button ~disabled:(not (N.can_pop routes)) "Back" (change t N.pop)
      ; button ~disabled:(not (N.can_forward routes)) "Forward" (change t N.forward)
      ; button
          ~accessible_name:"Workspace"
          (if compact then "Home" else "Workspace")
          (change t N.pop_to_root)
      ; button
          ~accessible_name:"Diagram"
          (if compact then "Run" else "Diagram")
          (navigate t Diagram)
      ]
  in
  V.panel
    ~key:(Gpuio.Key.of_string_exn "artifact-inspector")
    ~label:"Artifact workspace"
    ~active:opened
    ~hidden:Unmount
    ~style:
      (style
         [ Width (Gpuio.Length.percent_exn 100.)
         ; Height (Gpuio.Length.percent_exn 100.)
         ; Shrink 0.
         ; Min_height (px 0.)
         ; Background (Gpuio.Background.solid palette.sidebar)
         ; Foreground palette.text
         ; Border_left_width 1.
         ; Border_color palette.line
         ])
    [ Responsive.at_width
        ~key:"inspector-heading"
        ~height:45.
        ~breakpoint:420.
        ~compact:(heading ~compact:true)
        ~wide:(heading ~compact:false)
    ; V.column
        ~style:(style [ Padding (px 14.); Gap (px 10.); Shrink 0. ])
        [ Responsive.at_width
            ~key:"inspector-navigation"
            ~height:36.
            ~breakpoint:400.
            ~compact:(navigation ~compact:true)
            ~wide:(navigation ~compact:false)
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
