open Core
module C = Gpuio.Color_input
module V = Gpuio.Color_value
module W = Gpuio_protocol.Color_input_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let color text = V.Rgba.of_hex text |> ok
let labels = C.Labels.english ~control:"Accent" |> ok

let config () =
  C.Config.create
    ~labels
    ~read_only:true
    ~palette:
      [ C.Palette_entry.create ~color:(color "#ff000080") ~label:"Half red" |> ok
      ; C.Palette_entry.create ~color:(color "#112233ff") ~label:"Slate" |> ok
      ]
    ()
  |> ok
;;

let snapshot () : W.Snapshot.t =
  { revision = 8L
  ; value = Color 0xff000080L
  ; committed = Color 0x00ff00ffL
  ; channels = { hue_degrees = 0.; saturation = 1.; lightness = 0.5; alpha = 0.5 }
  ; interaction = Some { id = 5L; kind = Text Hex }
  ; draft = Some { text = "#ff000080"; composing = false; status = Valid }
  ; value_allowed = true
  ; committed_allowed = true
  }
;;

let fixture ~writer ~reader ~equal value name =
  let bytes = Bin_prot.Utils.bin_dump writer value in
  let encoded = Bigstring.to_string bytes in
  let hex =
    String.to_list encoded
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal hex (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip)));
  let pos_ref = ref 0 in
  assert (equal value (reader bytes ~pos_ref));
  assert (!pos_ref = String.length encoded);
  String.length encoded
;;

let%expect_test
    "independently assembled color configuration, preview, set and error fixtures"
  =
  let c = config () in
  let wire = C.Expert.config_to_wire c in
  assert (C.Config.equal c (C.Expert.config_of_wire wire |> ok));
  printf
    "config %d\n"
    (fixture
       ~writer:W.Config.bin_writer_t
       ~reader:W.Config.bin_read_t
       ~equal:W.Config.equal
       wire
       "color-config.hex");
  let e = W.Event.Preview (snapshot ()) in
  printf
    "preview %d\n"
    (fixture
       ~writer:W.Event.bin_writer_t
       ~reader:W.Event.bin_read_t
       ~equal:W.Event.equal
       e
       "color-preview.hex");
  let observed = C.Expert.event_of_wire ~window ~node e |> ok |> C.Event.snapshot in
  assert (Gpuio_protocol.Window_id.equal window (C.Expert.window observed));
  assert (Gpuio_protocol.Node_id.equal node (C.Expert.node observed));
  assert (V.Value.equal (C.Snapshot.value observed) (Color (color "#ff000080")));
  assert (V.Value.equal (C.Snapshot.committed observed) (Color (color "#00ff00")));
  assert (Float.equal (V.Hsla.alpha (C.Snapshot.channels observed)) 0.5);
  assert (Int64.equal (C.Snapshot.revision observed |> C.Revision.to_int64) 8L);
  let interaction = C.Snapshot.interaction observed |> Option.value_exn in
  assert (Int64.equal (C.Interaction.id interaction |> C.Interaction_id.to_int64) 5L);
  assert (C.Interaction.Kind.equal (C.Interaction.kind interaction) (Text Hex));
  let draft = C.Snapshot.draft observed |> Option.value_exn in
  assert (
    String.equal (C.Draft.text draft) "#ff000080" && not (C.Draft.is_composing draft));
  assert (C.Draft.Status.equal (C.Draft.status draft) Valid);
  let cmd =
    C.Command.Set
      { value = Color (color "#11223344")
      ; if_revision = Some (C.Revision.of_int64 7L |> ok)
      }
    |> C.Expert.command_to_wire
  in
  printf
    "set %d\n"
    (fixture
       ~writer:W.Command.bin_writer_t
       ~reader:W.Command.bin_read_t
       ~equal:W.Command.equal
       cmd
       "color-set.hex");
  printf
    "failed %d\n"
    (fixture
       ~writer:W.Response.bin_writer_t
       ~reader:W.Response.bin_read_t
       ~equal:W.Response.equal
       (Failed Stale_interaction)
       "color-failed.hex");
  [%expect
    {|
    config 96
    preview 69
    set 9
    failed 2
  |}]
;;

let%expect_test "configuration guards and historical value policy are explicit" =
  List.iter
    [ ""; " \t"; "bad\000label"; "\255"; String.make 4097 'x' ]
    ~f:(fun control -> assert (Result.is_error (C.Labels.english ~control)));
  List.iter
    [ ""; " \t"; "bad\000label"; "\255"; String.make 257 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (C.Palette_entry.create ~color:(color "#123") ~label)));
  let entry = C.Palette_entry.create ~color:(color "#1234") ~label:"Translucent" |> ok in
  assert (V.Rgba.equal (C.Palette_entry.color entry) (color "#1234"));
  assert (String.equal (C.Palette_entry.label entry) "Translucent");
  let palette = List.init 256 ~f:(fun _ -> entry) in
  let c =
    C.Config.create ~labels ~palette ~alpha_policy:Opaque_only ~allow_empty:false () |> ok
  in
  assert (not (C.Config.allows c (Color (color "#1234"))));
  assert (not (C.Config.allows c Empty));
  assert (C.Config.allows c (Color (color "#123")));
  assert (Result.is_error (C.Config.create ~labels ~palette:(entry :: palette) ()));
  assert ((not (C.Config.is_disabled c)) && not (C.Config.is_read_only c));
  assert (C.Config.is_read_only (config ()));
  assert (Result.is_error (C.Revision.of_int64 (-1L)));
  let wire = C.Expert.config_to_wire c in
  let invalid = { wire with palette = [ { color = -1L; label = "Bad" } ] } in
  assert (Result.is_error (C.Expert.config_of_wire invalid));
  print_endline
    "UTF-8/labels/palette/revision guards; opaque policy does not coerce colors";
  [%expect
    {| UTF-8/labels/palette/revision guards; opaque policy does not coerce colors |}]
;;

let%expect_test "snapshot and event import rejects contradictions before public accessors"
  =
  let s = snapshot () in
  let idle = { s with committed = s.value; interaction = None; draft = None } in
  assert (Result.is_ok (C.Expert.snapshot_of_wire ~window ~node idle));
  List.iter
    [ { s with revision = -1L }
    ; { s with value = Color (-1L) }
    ; { s with committed = Color 0x100000000L }
    ; { s with value = Color 0xff0000ffL }
    ; { s with channels = { s.channels with alpha = Float.nan } }
    ; { s with channels = { s.channels with saturation = 1.1 } }
    ; { s with interaction = Some { id = 9L; kind = Text Hex } }
    ; { s with interaction = Some { id = 0L; kind = Text Hex } }
    ; { s with interaction = Some { id = 5L; kind = Drag Hue } }
    ; { s with interaction = None }
    ; { s with draft = None }
    ; { s with draft = Some { text = "oops"; composing = false; status = Valid } }
    ; { s with draft = Some { text = "a\000b"; composing = false; status = Invalid } }
    ; { s with
        draft = Some { text = String.make 4097 'x'; composing = false; status = Invalid }
      }
    ; { idle with committed_allowed = false }
    ]
    ~f:(fun data ->
      assert (not (W.Snapshot.valid data));
      assert (Result.is_error (C.Expert.snapshot_of_wire ~window ~node data)));
  List.iter
    [ W.Event.Started s; Preview idle; Committed (Text, s); Cancelled (Escape, s) ]
    ~f:(fun e -> assert (Result.is_error (C.Expert.event_of_wire ~window ~node e)));
  let started =
    { s with committed = s.value; interaction = Some { id = 8L; kind = Text Hex } }
  in
  List.iter
    [ W.Event.Started started
    ; Observed s
    ; Preview s
    ; Committed (Text, idle)
    ; Cancelled (Escape, idle)
    ]
    ~f:(fun e -> assert (Result.is_ok (C.Expert.event_of_wire ~window ~node e)));
  List.iter
    [ C.Command.Reset { if_revision = None }
    ; Cancel
    ; Focus (Channel Alpha)
    ; Read_snapshot
    ]
    ~f:(fun c -> assert (W.Command.valid (C.Expert.command_to_wire c)));
  print_endline
    "identities, channel/value agreement, draft classification and lifecycle shapes \
     validated";
  [%expect
    {| identities, channel/value agreement, draft classification and lifecycle shapes validated |}]
;;

let%expect_test
    "wire HSLA validation agrees with public conversion and bounded maximum config"
  =
  let module H = V.Hsla in
  for hue = 0 to 360 do
    List.iter [ 0.; 0.25; 0.5; 0.75; 1. ] ~f:(fun lightness ->
      let h =
        H.create ~hue_degrees:(Float.of_int hue) ~saturation:0.75 ~lightness ~alpha:0.5
        |> ok
      in
      let wire : W.Hsla.t =
        { hue_degrees = H.hue_degrees h
        ; saturation = H.saturation h
        ; lightness = H.lightness h
        ; alpha = H.alpha h
        }
      in
      match C.Expert.value_to_wire (Color (V.Rgba.of_hsla h)) with
      | Empty -> assert false
      | Color packed -> assert (Int64.equal packed (W.Hsla.rgba wire)))
  done;
  let text = String.make 4096 'L' in
  let labels =
    C.Labels.create
      ~control:text
      ~hue:text
      ~saturation:text
      ~lightness:text
      ~alpha:text
      ~hex:text
      ~clear:text
    |> ok
  in
  let entry =
    C.Palette_entry.create ~color:(color "#ffffffff") ~label:(String.make 256 'P') |> ok
  in
  let c =
    C.Config.create ~labels ~palette:(List.init 256 ~f:(fun _ -> entry)) ()
    |> ok
    |> C.Expert.config_to_wire
  in
  let encoded = Bin_prot.Utils.bin_dump W.Config.bin_writer_t c in
  assert (Bigstring.length encoded <= W.max_config_bytes);
  printf "1805 channel cases; maximum config %d bytes\n" (Bigstring.length encoded);
  [%expect {| 1805 channel cases; maximum config 97308 bytes |}]
;;

let%expect_test "color input capability uses the shared 64-bit handshake" =
  let module Wire = Gpuio_protocol.Wire in
  assert (Int64.equal (Int64.bit_and Wire.capabilities 137438953472L) 137438953472L);
  let bytes =
    Wire.Message.encode (Hello (Wire.version, Wire.capabilities)) |> Or_error.ok_exn
  in
  String.iter bytes ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0001fcffffffffff1f0000 |}]
;;
