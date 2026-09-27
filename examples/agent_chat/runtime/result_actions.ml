open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module D = Gpuio.Table_data
module T = Gpuio.Table
module W = Gpuio_bonsai.Table
module View = Gpuio_bonsai.View

module Target = struct
  type t =
    { row : D.Row_ref.t
    ; generation : int64
    ; session : Type_equal.Id.Uid.t
    }
  [@@deriving equal]
end

type t = Target.t option B.Expert.Var.t

let create () = B.Expert.Var.create None

let request t ~generation request =
  E.of_thunk (fun () ->
    match request with
    | T.Request.Activate (row, _) | Context (Row row | Cell (row, _)) ->
      let session =
        Type_equal.Id.create ~name:"event actions" sexp_of_unit |> Type_equal.Id.uid
      in
      B.Expert.Var.set t (Some { Target.row; generation; session })
    | Context (Empty | Column _) -> B.Expert.Var.set t None
    | Select _ | Resize _ | Move _ | Sort _ | Copy _ -> ())
;;

let valid (target : Target.t) (snapshot : (_, _) Gpuio.Table_paging.Snapshot.t) =
  Int64.equal target.generation snapshot.generation
  && D.contains_ref snapshot.data target.row
;;

let view t ~snapshot ~current ~output ~describe ~result_column ~dark graph =
  let is_current target = Option.exists (B.Expert.Var.get t) ~f:(Target.equal target) in
  let close_target target =
    E.of_thunk (fun () -> if is_current target then B.Expert.Var.set t None)
  in
  let open B.Let_syntax in
  let target = B.Expert.Var.value t in
  B.Edge.after_display
    (let%arr target = target
     and snapshot = snapshot in
     match target with
     | Some target when not (valid target snapshot) -> close_target target
     | Some _ | None -> E.Ignore)
    graph;
  let%arr target = target
  and snapshot = snapshot
  and output = output
  and dark = dark in
  let palette = Palette.of_dark dark in
  let button label on_click =
    View.button
      label
      ~on_click
      ~style:
        (Gpuio.Style.create_exn
           [ Foreground palette.text
           ; Background (Gpuio.Background.solid palette.raised)
           ; Border_width 1.
           ; Border_color palette.line
           ; Padding (Gpuio.Length.px_exn 9.)
           ; Radius 8.
           ])
  in
  let close = Option.value_map target ~default:E.Ignore ~f:close_target in
  let content =
    Option.bind target ~f:(fun target ->
      if not (valid target snapshot)
      then None
      else
        Option.bind
          (D.find snapshot.data (D.Row_ref.id target.row))
          ~f:(fun row ->
            Option.map (Or_error.ok output) ~f:(fun output ->
              let controller = W.Output.controller output in
              let reveal =
                E.bind
                  (E.of_thunk (fun () -> is_current target && valid target (current ())))
                  ~f:(fun live ->
                    if not live
                    then E.Ignore
                    else
                      E.Many
                        [ W.Controller.batch
                            controller
                            [ Set_selection (Cell (target.row, result_column))
                            ; Reveal (target.row, Some result_column)
                            ]
                          |> Or_error.ok_exn
                        ; close
                        ])
              in
              View.column
                ~style:
                  (Gpuio.Style.create_exn
                     [ Padding (Gpuio.Length.px_exn 20.); Gap (Gpuio.Length.px_exn 16.) ])
                [ View.text
                    ~style:(Gpuio.Style.create_exn [ Font_size 22. ])
                    "Finding details"
                ; View.text
                    ~style:(Gpuio.Style.create_exn [ User_select true ])
                    (describe row)
                ; View.text
                    "Reveal the result, then use Copy to copy its complete Unicode text."
                ; View.row
                    ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 12.) ])
                    [ button "Close finding details" close
                    ; button "Reveal finding" reveal
                    ]
                ])))
  in
  View.dialog
    ~style:
      (Gpuio.Style.create_exn
         [ Background (Gpuio.Background.solid palette.surface)
         ; Foreground palette.text
         ; Border_color palette.line
         ])
    ~config:
      (Gpuio.Overlay.Config.create ~label:"Finding actions" ~width:560. ()
       |> Or_error.ok_exn)
    ~on_dismiss:(fun _ -> close)
    content
;;
