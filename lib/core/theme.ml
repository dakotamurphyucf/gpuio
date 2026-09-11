open Core
type t = int64 String.Map.t [@@deriving equal, sexp_of]
let create definitions =
  List.fold_result definitions ~init:String.Map.empty ~f:(fun map (name,color) ->
    let%bind.Or_error _ = Color.token name in
    match Color.Expert.value color with
    | Token _ -> Or_error.error_string "theme definitions must be concrete colors"
    | Rgba value ->
      match Map.add map ~key:name ~data:value with
      | `Duplicate -> Or_error.errorf "duplicate theme token %s" name
      | `Ok map -> Ok map)
;;
let default = create ["background",Color.rgb_exn 0x172136;"foreground",Color.rgb_exn 0xf1f5ff;"accent",Color.rgb_exn 0x386ac8;"muted",Color.rgb_exn 0x8996ab] |> Or_error.ok_exn
let resolve t color = match Color.Expert.value color with
  | Rgba value -> Ok value
  | Token name -> (match Map.find t name with Some value -> Ok value | None -> Or_error.errorf "undefined theme token %s" name)
;;
