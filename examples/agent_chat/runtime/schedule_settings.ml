open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module C = Gpuio.Calendar
module Picker = Gpuio_eio.Date_picker
module Data = Schedule_data
module Pages = Gpuio.Pagination
module P = Gpuio.Presentation

module Model = struct
  type t =
    { mode : C.Mode.t
    ; draft : C.Selection.t
    ; applied : C.Selection.t
    ; month : C.Month.t
    ; epoch : int
    ; pages : Pages.t
    ; notice : string
    }
end

type t =
  { model : Model.t B.Expert.Var.t
  ; follow_up : C.Selection.t B.Expert.Var.t
  }

let ok = Or_error.ok_exn
let px = Gpuio.Length.px_exn
let style = Gpuio.Style.create_exn
let key = Gpuio.Key.of_string_exn
let total_pages count = (count + 5) / 6

let create () =
  { model =
      B.Expert.Var.create
        { Model.mode = Range
        ; draft = C.Selection.empty
        ; applied = C.Selection.empty
        ; month = Data.month
        ; epoch = 0
        ; pages =
            Pages.create
              ~total_pages:
                (total_pages (Data.reviews C.Selection.empty |> ok |> List.length))
              ()
            |> ok
        ; notice = "Choose a day or interval to filter the sample reviews."
        }
  ; follow_up = B.Expert.Var.create C.Selection.empty
  }
;;

let component t ~window ~is_current ~dark graph =
  let open B.Let_syntax in
  let picker =
    Picker.create
      window
      ~config:(B.return (Data.config ~mode:Single ~label:"Follow-up date"))
      ~value:(B.Expert.Var.value t.follow_up)
      ~initial_month:Data.month
      ~on_change:
        (B.map is_current ~f:(fun current selection ->
           E.of_thunk (fun () ->
             if current () then B.Expert.Var.set t.follow_up selection)))
      graph
  in
  let%arr model = B.Expert.Var.value t.model
  and follow_up = B.Expert.Var.value t.follow_up
  and picker = picker
  and current = is_current
  and dark = dark in
  let p = Palette.of_dark dark in
  let appearance = if dark then P.Appearance.dark else P.Appearance.light in
  let mutate f =
    E.of_thunk (fun () ->
      if current () then B.Expert.Var.set t.model (f (B.Expert.Var.get t.model)))
  in
  let button ?(disabled = false) label on_click =
    V.button
      ~disabled
      ~on_click
      label
      ~style:
        (style
           [ Foreground p.text
           ; Background (Gpuio.Background.solid p.raised)
           ; Padding (px 8.)
           ; Radius 8.
           ; Border_width 1.
           ; Border_color p.line
           ])
  in
  let text value =
    V.text
      ~style:(style [ Foreground p.muted; Font_size 12.; Line_height (px 18.) ])
      value
  in
  let apply model selection =
    match Data.reviews selection with
    | Error error -> { model with Model.notice = Error.to_string_hum error }
    | Ok reviews ->
      { model with
        applied = selection
      ; pages =
          Pages.with_total_pages model.pages (total_pages (List.length reviews)) |> ok
      ; notice = "Review date filter applied."
      }
  in
  let calendar =
    Gpuio.View.calendar
      ~controller:(key (sprintf "review-calendar-%d" model.epoch))
      ~config:(Data.config ~mode:model.mode ~label:"Review dates")
      ~initial:model.draft
      ~initial_month:model.month
      ~style:(style [ Foreground p.text; Background (Gpuio.Background.solid p.surface) ])
      ~on_event:(fun event ->
        mutate (fun latest ->
          if latest.epoch <> model.epoch
          then latest
          else (
            match event with
            | C.Event.Observed _ -> latest
            | Changed snapshot | Selected snapshot ->
              { latest with
                draft = C.Snapshot.selection snapshot
              ; month = C.Snapshot.month snapshot
              }
            | Rejected (_, _) ->
              { latest with notice = "Choose available October 2026 endpoints." })))
      ()
  in
  let reviews = Data.reviews model.applied |> ok in
  let page = Option.value (Pages.current model.pages) ~default:1 in
  let visible = List.drop reviews ((page - 1) * 6) |> Fn.flip List.take 6 in
  let pagination =
    Gpuio.Navigation.pagination
      model.pages
      ~labels:
        (Gpuio.Navigation.Pagination_labels.create
           ~navigation:"Sample review pages"
           ~first:"First reviews"
           ~previous:"Previous reviews"
           ~next:"Next reviews"
           ~last:"Last reviews"
           ~current:"Current review page"
           ~page:(sprintf "Review page %d")
           ~gap:(fun ~first ~last -> sprintf "Review pages %d–%d" first last)
         |> ok)
      ~appearance:
        (Gpuio.Navigation.Appearance.create
           ~item_style:
             (style
                [ Foreground p.text
                ; Background (Gpuio.Background.solid p.raised)
                ; Border_color p.line
                ; Padding (px 6.)
                ; Font_size 12.
                ])
           ~current_style:
             (style
                [ Foreground p.accent
                ; Background (Gpuio.Background.solid p.accent_surface)
                ])
           ())
      ~on_request:(fun request ->
        mutate (fun model ->
          { model with pages = Pages.apply_request model.pages request }))
      ()
    |> ok
  in
  V.column
    ~style:(style [ Gap (px 18.) ])
    [ P.settings_group
        appearance
        ~title:"Leave room for a follow-up"
        ~description:
          "A civil date for a simulated task. This does not schedule or send anything."
        [ Picker.view
            picker
            ~label:"Choose follow-up date"
            ~apply_label:"Save follow-up"
            ~cancel_label:"Cancel follow-up"
            ~overlay:
              (Gpuio.Overlay.Config.create ~label:"Choose follow-up date" ~width:340. ()
               |> ok)
            ~style:
              (style
                 [ Background (Gpuio.Background.solid p.surface)
                 ; Foreground p.text
                 ; Border_color p.line
                 ])
        ; text ("Saved follow-up: " ^ Data.describe follow_up)
        ; (match Picker.error picker with
           | None -> V.column []
           | Some _ ->
             text "This date could not be saved. Choose an available day and try again.")
        ; button
            "Clear follow-up"
            (E.Many
               [ Picker.cancel picker
               ; E.of_thunk (fun () ->
                   if current () then B.Expert.Var.set t.follow_up C.Selection.empty)
               ])
        ]
    ; P.settings_group
        appearance
        ~title:"Review a little at a time"
        ~description:
          "Sample October 2026. Endpoints must be weekdays; October 20 is unavailable."
        [ V.row
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            (List.map
               [ C.Mode.Single, "Single review day"; Range, "Review date range" ]
               ~f:(fun (mode, label) ->
                 button
                   ~disabled:(C.Mode.equal mode model.mode)
                   label
                   (mutate (fun model ->
                      { model with
                        mode
                      ; draft = C.Selection.empty
                      ; epoch = model.epoch + 1
                      }))))
        ; calendar
        ; text ("Date draft: " ^ Data.describe model.draft)
        ; V.row
            ~style:(style [ Gap (px 8.); Wrap Wrap ])
            [ button
                ~disabled:(Result.is_error (Data.reviews model.draft))
                "Apply review dates"
                (mutate (fun model -> apply model model.draft))
            ; button
                "All review dates"
                (mutate (fun model ->
                   let model = apply model C.Selection.empty in
                   { model with draft = C.Selection.empty; epoch = model.epoch + 1 }))
            ]
        ; text
            (sprintf
               "%d sample review%s · page %d of %d"
               (List.length reviews)
               (if List.length reviews = 1 then "" else "s")
               page
               (Pages.total_pages model.pages))
        ; P.description_list
            appearance
            ~stacked:true
            (List.map visible ~f:(fun review ->
               P.Description.create
                 ~key:(key (Date.to_string (Data.Review.date review)))
                 ~term:(Date.to_string (Data.Review.date review))
                 ~definition:(text (Data.Review.label review))))
        ; pagination
        ; text model.notice
        ]
    ]
;;
