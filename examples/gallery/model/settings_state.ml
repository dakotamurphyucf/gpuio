open Core
module S = Gpuio.Settings
module N = Gpuio.Number_input

let ok = Or_error.ok_exn
let field_id name = S.Item_id.of_string name |> ok
let page_id name = S.Page_id.of_string name |> ok
let group_id name = S.Group_id.of_string name |> ok

module Field = struct
  type t =
    | Name
    | Notifications
    | Reports
    | Budget
    | Region
    | Model
    | Custom
    | Locked
    | Feature of int
  [@@deriving equal, compare, sexp_of]

  let all =
    [ Name; Notifications; Reports; Budget; Region; Model; Custom; Locked ]
    @ List.init 48 ~f:(fun n -> Feature n)
  ;;

  let key = function
    | Name -> "workspace-name"
    | Notifications -> "notifications"
    | Reports -> "reports"
    | Budget -> "budget"
    | Region -> "region"
    | Model -> "model"
    | Custom -> "custom"
    | Locked -> "locked"
    | Feature n -> sprintf "feature-%02d" n
  ;;

  let id t = field_id (key t)
  let of_id id = List.find all ~f:(fun t -> S.Item_id.equal id (field_id (key t)))
end

type values =
  { name : string
  ; notifications : bool
  ; reports : bool
  ; budget : N.Value.t
  ; budget_draft : N.Draft.t
  ; region : int
  ; model : int
  ; custom : int
  ; features : Int.Set.t
  ; locked : bool
  }
[@@deriving equal]

type t =
  { values : values
  ; catalog : S.t
  }
[@@deriving equal]

let defaults =
  { name = "Northstar"
  ; notifications = true
  ; reports = false
  ; budget = N.Value.of_float 25. |> ok
  ; budget_draft = N.Draft.of_string "25" |> ok
  ; region = 0
  ; model = 0
  ; custom = 0
  ; features = Int.Set.empty
  ; locked = true
  }
;;

let dirty v = function
  | Field.Name -> not (String.equal v.name defaults.name)
  | Notifications -> not (Bool.equal v.notifications defaults.notifications)
  | Reports -> not (Bool.equal v.reports defaults.reports)
  | Budget ->
    not
      (N.Value.equal v.budget defaults.budget
       && N.Draft.equal v.budget_draft defaults.budget_draft)
  | Region -> v.region <> defaults.region
  | Model -> v.model <> defaults.model
  | Custom -> v.custom <> defaults.custom
  | Locked -> false
  | Feature n -> Set.mem v.features n
;;

let disabled v = function
  | Field.Locked -> true
  | Custom -> v.locked
  | _ -> false
;;

let pages v =
  let item ?(vertical = false) field title description =
    S.Item.create
      ~id:(Field.id field)
      ~title
      ~description
      ~layout:(if vertical then Vertical else Horizontal)
      ~disabled:(disabled v field)
      ~reset:(if dirty v field then Dirty else Clean)
      ()
    |> ok
  in
  let group name title items = S.Group.create ~id:(group_id name) ~title items |> ok in
  let page name title groups =
    S.Page.create
      ~id:(page_id name)
      ~title
      ~default_open:(String.equal name "workspace")
      groups
    |> ok
  in
  [ page
      "workspace"
      "Workspace"
      [ group
          "identity"
          "Make it yours"
          [ item Field.Name "Workspace name" "A name for this workspace."
          ; item Notifications "Desktop notifications" "Keep track of completed work."
          ; item Reports "Weekly summaries" "A quiet digest of your activity."
          ]
      ; group
          "generation"
          "Generation"
          [ item
              Budget
              "Response budget"
              "A committed value and a separate editable draft."
          ; item Region "Preferred region" "Keep requests close to your team."
          ; item
              ~vertical:true
              Model
              "Default model"
              "Choose from 250 stable, typed model IDs."
          ]
      ; group
          "policy"
          "Managed preferences"
          [ S.Item.custom
              ~id:(Field.id Custom)
              ~keywords:[ "custom"; "policy"; "counter" ]
              ~disabled:v.locked
              ~reset:(if dirty v Custom then Dirty else Clean)
              ()
            |> ok
          ; item
              Locked
              "Organization policy"
              "This preference is managed by your organization."
          ]
      ]
  ; page
      "advanced"
      "Advanced"
      (List.init 48 ~f:(fun n ->
         group
           (sprintf "advanced-%02d" n)
           (sprintf "Experiment %02d" n)
           [ item
               (Feature n)
               (sprintf "Enable feature %02d" n)
               "Application data survives row eviction."
           ]))
  ]
;;

let initial = { values = defaults; catalog = S.create (pages defaults) |> ok }
let catalog t = t.catalog
let name t = t.values.name
let notifications t = t.values.notifications
let reports t = t.values.reports
let budget t = t.values.budget
let budget_draft t = t.values.budget_draft
let region t = t.values.region
let model t = t.values.model
let custom t = t.values.custom
let feature t n = Set.mem t.values.features n
let locked t = t.values.locked

module Action = struct
  type t =
    | Navigate of S.Request.t
    | Name of string
    | Toggle_notifications
    | Toggle_reports
    | Budget of N.Value.t * N.Draft.t
    | Region of int
    | Model of int
    | Custom
    | Toggle_feature of int
    | Toggle_lock
    | Reset of Field.t
end

let reset_targets t scope =
  S.reset_targets t.catalog ~scope |> List.filter_map ~f:Field.of_id
;;

let can_reset t field =
  List.exists (reset_targets t (Item (Field.id field))) ~f:(Field.equal field)
;;

let reset v = function
  | Field.Name -> { v with name = defaults.name }
  | Notifications -> { v with notifications = defaults.notifications }
  | Reports -> { v with reports = defaults.reports }
  | Budget -> { v with budget = defaults.budget; budget_draft = defaults.budget_draft }
  | Region -> { v with region = defaults.region }
  | Model -> { v with model = defaults.model }
  | Custom -> { v with custom = 0 }
  | Locked -> v
  | Feature n -> { v with features = Set.remove v.features n }
;;

let apply t action =
  let open Or_error.Let_syntax in
  match action with
  | Action.Navigate request -> Ok { t with catalog = S.apply_request t.catalog request }
  | _ ->
    let%bind values =
      let v = t.values in
      match action with
      | Navigate _ -> assert false
      | Name name ->
        let%bind () = Gpuio.Text_input.validate_text ~mode:Single_line name in
        Ok { v with name }
      | Toggle_notifications -> Ok { v with notifications = not v.notifications }
      | Toggle_reports -> Ok { v with reports = not v.reports }
      | Budget (budget, budget_draft) ->
        (match budget with
         | N.Value.Empty -> Or_error.error_string "A budget is required"
         | Number n when (not (Float.is_finite n)) || Float.(n < 0. || n > 1000.) ->
           Or_error.error_string "Budget outside 0..1000"
         | Number _ -> Ok { v with budget; budget_draft })
      | Region region ->
        if region < 0 || region >= 3
        then Or_error.error_string "Unknown region"
        else Ok { v with region }
      | Model model ->
        if model < 0 || model >= 250
        then Or_error.error_string "Unknown model"
        else Ok { v with model }
      | Custom -> Ok (if v.locked then v else { v with custom = v.custom + 1 })
      | Toggle_feature n ->
        if n < 0 || n >= 48
        then Or_error.error_string "Unknown feature"
        else
          Ok
            { v with
              features =
                (if Set.mem v.features n
                 then Set.remove v.features n
                 else Set.add v.features n)
            }
      | Toggle_lock -> Ok { v with locked = not v.locked }
      | Reset field -> Ok (if can_reset t field then reset v field else v)
    in
    if equal_values t.values values
    then Ok t
    else (
      let%map catalog = S.with_pages t.catalog (pages values) in
      { values; catalog })
;;

let encode t =
  let v = t.values in
  Sexp.to_string_hum
    [%sexp
      ("gpuio-settings-preview-v1" : string)
    , (v.name : string)
    , (v.notifications : bool)
    , (v.reports : bool)
    , (v.budget : N.Value.t)
    , (v.region : int)
    , (v.model : int)
    , (v.custom : int)
    , (Set.to_list v.features : int list)]
  ^ "\n"
;;
