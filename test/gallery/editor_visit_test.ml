open Core
module V = Gpuio_gallery_model.Editor_visit
module M = Gpuio_gallery_model.Settings_state

let ok = Or_error.ok_exn

let%expect_test
    "retired replies cannot undo an unmounted reset or a new native revision zero"
  =
  let value = ref M.initial in
  let observe visit revision text =
    if V.observe visit ~revision then value := M.apply !value (Name text) |> ok
  in
  let old = V.create () in
  observe old 0L "not mounted";
  assert (M.equal !value M.initial);
  V.activate old;
  observe old 8L "edited";
  observe old 7L "late older event";
  observe old 8L "duplicate";
  assert (String.equal (M.name !value) "edited");
  V.deactivate old;
  value := M.apply !value (Reset Name) |> ok;
  observe old 9L "late command reply";
  assert (String.equal (M.name !value) "Northstar");
  let current = V.create () in
  V.activate current;
  observe current 0L "new visit";
  observe old 99L "old visit after remount";
  observe current (-1L) "invalid revision";
  assert (String.equal (M.name !value) "new visit");
  observe current 2L "later input";
  observe current 1L "earlier reset reply";
  assert (String.equal (M.name !value) "later input");
  assert (Result.is_error (Or_error.try_with (fun () -> V.activate old)));
  print_endline
    "old visits, old revisions and invalid reactivation cannot overwrite newer \
     application data";
  [%expect
    {| old visits, old revisions and invalid reactivation cannot overwrite newer application data |}]
;;
