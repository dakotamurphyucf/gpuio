open Core
module T = Gpuio.Tree
module S = Gpuio.Tree_state
module L = Gpuio.Tree_loading
module I = Gpuio.Tree_interaction
module Search = Gpuio.Tree_typeahead

let ok = Or_error.ok_exn
let id name = T.Id.of_string name |> ok

let leaf ?(disabled = false) label =
  T.Node.create ~label ~disabled ~children:Leaf () |> ok
;;

let collection () =
  let nodes =
    [ "alpha", leaf "Alpha"
    ; "disabled", leaf ~disabled:true "Álgebra"
    ; "alpine", leaf "Alpine"
    ; "eclair", leaf "Éclair"
    ; "ecole", leaf "École"
    ; "sharp", leaf "Straße"
    ; "upper", leaf "STRASSE"
    ; "sigma", leaf "Σίσυφος"
    ; "sig", leaf "σίγμα"
    ; "family", leaf "👨‍👩‍👧‍👦 Home"
    ]
    |> List.map ~f:(fun (name, node) -> id name, node)
  in
  T.create ~roots:(List.map nodes ~f:fst) nodes |> ok
;;

let input ?(reset = false) ?(cycle = true) text =
  Search.Input.create ~reset ~cycle text |> ok
;;

let active state = S.active state |> Option.map ~f:T.Id.to_string

let%expect_test "Unicode prefixes cycle, extend, wrap and canonically match" =
  let tree = collection () in
  let source = L.snapshot (L.create tree) in
  let state = ref (S.create tree ~mode:Multiple () |> ok) in
  let step ?reset ?cycle text =
    let outcome =
      I.apply !state source (I.Request.typeahead source (input ?reset ?cycle text))
      |> Option.value_exn
    in
    state := I.Outcome.state outcome;
    (match I.Outcome.action outcome with
     | None -> ()
     | Activate _ | Move _ -> assert false);
    print_s [%sexp (active !state : string option), (I.Outcome.focus outcome : bool)]
  in
  step ~reset:true "a";
  step "A";
  step "a";
  step "l";
  step "p";
  step "i";
  step "x";
  step ~reset:true "É";
  step "é";
  step "c";
  step "o";
  step ~reset:true ~cycle:false "Straße";
  step ~reset:true ~cycle:false "STRASSE";
  step ~reset:true "σ";
  step "ί";
  step "γ";
  step ~reset:true "👨‍👩‍👧‍👦";
  step ~reset:true "e";
  step "́";
  [%expect
    {|
    ((alpha) true)
    ((alpine) true)
    ((alpha) true)
    ((alpha) true)
    ((alpha) true)
    ((alpine) true)
    ((alpine) false)
    ((eclair) true)
    ((ecole) true)
    ((ecole) true)
    ((ecole) true)
    ((sharp) true)
    ((upper) true)
    ((sigma) true)
    ((sigma) true)
    ((sig) true)
    ((family) true)
    ((eclair) true)
    ((eclair) true)
  |}]
;;

let%expect_test "canonical search resets after an early mismatch and orders marks" =
  let nodes =
    [ id "miss", leaf ("Ω" ^ String.make 4000 'x')
    ; id "base", leaf "Åland"
    ; id "first", leaf "Å\204\163land"
    ; id "second", leaf "A\204\163\204\138land"
    ; id "ligature", leaf "ﬃle"
    ]
  in
  let tree = T.create ~roots:(List.map nodes ~f:fst) nodes |> ok in
  let state = ref (S.create tree () |> ok) in
  let search ?reset text =
    let next, found = S.typeahead !state tree (input ?reset text) in
    state := next;
    print_s [%sexp (Option.map found ~f:T.Id.to_string : string option)]
  in
  search ~reset:true "a";
  search "\204\138";
  search "\204\163";
  search ~reset:true "å\204\163";
  search ~reset:true "FFI";
  search ~reset:true "ﬃ";
  [%expect
    {|
    (base)
    (base)
    (first)
    (second)
    (ligature)
    (ligature)
    |}]
;;

let%expect_test "search uses current visible labels and stale source requests retire" =
  let child = id "child" in
  let folder = id "folder" in
  let make label =
    [ ( folder
      , T.Node.create
          ~label:"Folder"
          ~children:(Branch { ids = [ child ]; next = End })
          ()
        |> ok )
    ; child, leaf label
    ]
  in
  let tree = T.create ~roots:[ folder ] (make "Étoile") |> ok in
  let state = S.create tree () |> ok in
  let state, found = S.typeahead state tree (input ~reset:true "é") in
  assert (Option.is_none found);
  let state = S.toggle_expanded state tree folder in
  let state, found = S.typeahead state tree (input ~reset:true "é") in
  assert (Option.equal T.Id.equal found (Some child));
  let changed = T.replace tree ~roots:[ folder ] (make "Delta") |> ok in
  let state, found = S.typeahead state changed (input ~reset:true "é") in
  assert (Option.is_none found);
  let _, found = S.typeahead state changed (input ~reset:true "d") in
  assert (Option.equal T.Id.equal found (Some child));
  let source = L.snapshot (L.create changed) in
  let old = I.Request.typeahead source (input ~reset:true "d") in
  let replacement = L.snapshot (L.create changed) in
  assert (Option.is_none (I.apply state replacement old));
  print_endline
    "hidden children skipped; current labels searched; foreign source rejected";
  [%expect
    {| hidden children skipped; current labels searched; foreign source rejected |}]
;;

let%expect_test "input and prefix bounds reject malformed text and restart overflow" =
  List.iter
    [ ""
    ; "\000"
    ; "\t"
    ; "\xc2\x85"
    ; "\xff"
    ; String.make 257 'a'
    ; String.concat (List.init 100 ~f:(fun _ -> "ΐ"))
    ]
    ~f:(fun text ->
      assert (Or_error.is_error (Search.Input.create ~reset:false ~cycle:true text)));
  let tree = T.create ~roots:[] [] |> ok in
  let search = ref Search.empty in
  for _ = 1 to 256 do
    let next, found =
      Search.advance !search tree ~visible:[] ~active:None (input ~cycle:false "e")
    in
    search := next;
    assert (Option.is_none found)
  done;
  (* An unmatched extension falls back immediately, keeping only the last input. *)
  assert (Search.prefix_bytes !search = 1);
  let nodes = [ id "long", leaf (String.make 4096 'e') ] in
  let tree = T.create ~roots:[ id "long" ] nodes |> ok in
  for _ = 1 to 600 do
    let next, _ =
      Search.advance
        !search
        tree
        ~visible:(T.roots tree)
        ~active:None
        (input ~cycle:false "e")
    in
    search := next;
    assert (Search.prefix_bytes !search <= 256)
  done;
  print_endline "invalid UTF-8/control/oversized input rejected; prefix remains bounded";
  [%expect {| invalid UTF-8/control/oversized input rejected; prefix remains bounded |}]
;;

let%expect_test
    "large loaded order finds an offscreen item and wraps without a label index"
  =
  let count = 100_000 in
  let nodes =
    List.init count ~f:(fun i ->
      id (Int.to_string i), leaf (if i = count - 1 then "Ωmega" else "row"))
  in
  let tree = T.create ~roots:(List.map nodes ~f:fst) nodes |> ok in
  let state = S.create tree () |> ok in
  let state, found = S.typeahead state tree (input ~reset:true "ω") in
  assert (Option.equal T.Id.equal found (Some (id "99999")));
  let state, found = S.typeahead state tree (input ~reset:true "R") in
  assert (Option.equal T.Id.equal found (Some (id "0")));
  assert (List.length (S.selected state) = 1);
  let _, found = S.typeahead state tree (input ~reset:true "not present") in
  assert (Option.is_none found);
  print_endline "100000 loaded nodes: last match, wrap to first and complete miss";
  [%expect {| 100000 loaded nodes: last match, wrap to first and complete miss |}]
;;
