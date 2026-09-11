open Core
module Kind = struct type t = Container | Text | Button [@@deriving equal, sexp_of] end
type 'action t =
 {key:Key.t option;kind:Kind.t;text:string;style:Style.t;on_click:(unit->'action) option;children:'action t list}
let text ?key ?(style=Style.empty) text = {key;kind=Text;text;style;on_click=None;children=[]}
let button ?key ?(style=Style.empty) ~on_click text =
 let defaults=Style.create_exn [Padding (Length.px_exn 8.); Radius 4.; Background (Background.solid (Color.token_exn "accent")); Foreground (Color.token_exn "foreground");Cursor Pointer] in
 {key;kind=Button;text;style=Style.merge [defaults;style];on_click=Some on_click;children=[]}
;;
let container ?key ?(style=Style.empty) defaults children =
 {key;kind=Container;text="";style=Style.merge [Style.create_exn defaults;style];on_click=None;children}
;;
let row ?key ?style children = container ?key ?style [Display Flex;Direction Row] children
let column ?key ?style children = container ?key ?style [Display Flex;Direction Column] children
let grid ?key ?style ~columns children =
 if columns < 1 || columns > 1024 then Or_error.error_string "grid columns must be in 1..1024"
 else Ok (container ?key ?style [Display Grid;Grid_columns columns] children)
;;
module Expert = struct
 module Kind = Kind
 type 'action description = 'action t =
  {key:Key.t option;kind:Kind.t;text:string;style:Style.t;on_click:(unit->'action) option;children:'action t list}
 let describe t = t
end
