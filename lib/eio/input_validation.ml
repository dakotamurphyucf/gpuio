open Core
module V = Gpuio.Input_validation

let preparation_lock = Eio.Mutex.create ()

let with_preparation_slot lock ~f =
  Eio.Fiber.check ();
  let result = Eio.Mutex.use_rw lock ~protect:true f in
  Eio.Fiber.check ();
  result
;;

let prepare_regex source =
  let result =
    with_preparation_slot preparation_lock ~f:(fun () ->
      Eio_unix.run_in_systhread ~label:"input regex preparation" (fun () ->
        Gpuio_native.prepare_input_regex (V.Expert.source_to_wire source)))
  in
  match result with
  | Error _ -> Error V.Error.Native_failure
  | Ok Checked -> Ok (V.Expert.checked_source source)
  | Ok (Failed error) -> Error (V.Expert.error_of_wire error)
;;

let%expect_test "cancellation retains the slot until admitted work finishes" =
  Eio_main.run (fun _ ->
    Eio.Switch.run (fun sw ->
      let lock = Eio.Mutex.create () in
      let started, started_resolver = Eio.Promise.create () in
      let release, release_resolver = Eio.Promise.create () in
      let finished, finished_resolver = Eio.Promise.create () in
      let waiting, waiting_resolver = Eio.Promise.create () in
      let second_finished, second_finished_resolver = Eio.Promise.create () in
      let work_finished = ref false in
      let second_entered = ref false in
      Eio.Fiber.fork ~sw (fun () ->
        let cancelled =
          try
            Eio.Cancel.sub (fun context ->
              with_preparation_slot lock ~f:(fun () ->
                Eio.Promise.resolve started_resolver context;
                Eio.Promise.await release;
                work_finished := true));
            false
          with
          | Eio.Cancel.Cancelled _ -> true
        in
        Eio.Promise.resolve finished_resolver cancelled);
      let context = Eio.Promise.await started in
      Eio.Cancel.cancel context Exit;
      Eio.Fiber.fork ~sw (fun () ->
        Eio.Promise.resolve waiting_resolver ();
        with_preparation_slot lock ~f:(fun () ->
          assert !work_finished;
          second_entered := true);
        Eio.Promise.resolve second_finished_resolver ());
      Eio.Promise.await waiting;
      assert (not !second_entered);
      assert (not !work_finished);
      assert (not (Eio.Promise.is_resolved finished));
      Eio.Promise.resolve release_resolver ();
      assert (Eio.Promise.await finished);
      Eio.Promise.await second_finished;
      assert !second_entered));
  print_endline "active work retains its slot; cancelled results are not delivered";
  [%expect {| active work retains its slot; cancelled results are not delivered |}]
;;

let%expect_test "a cancelled waiting caller does not start work or poison the slot" =
  Eio_main.run (fun _ ->
    Eio.Switch.run (fun sw ->
      let lock = Eio.Mutex.create () in
      let waiting, waiting_resolver = Eio.Promise.create () in
      let finished, finished_resolver = Eio.Promise.create () in
      let entered = ref false in
      Eio.Mutex.use_rw lock ~protect:true (fun () ->
        Eio.Fiber.fork ~sw (fun () ->
          let cancelled =
            try
              Eio.Cancel.sub (fun context ->
                Eio.Promise.resolve waiting_resolver context;
                with_preparation_slot lock ~f:(fun () -> entered := true));
              false
            with
            | Eio.Cancel.Cancelled _ -> true
          in
          Eio.Promise.resolve finished_resolver cancelled);
        Eio.Cancel.cancel (Eio.Promise.await waiting) Exit;
        assert (Eio.Promise.await finished);
        assert (not !entered));
      with_preparation_slot lock ~f:(fun () -> entered := true);
      assert !entered));
  print_endline "cancelled waiters never enter; the next caller can prepare";
  [%expect {| cancelled waiters never enter; the next caller can prepare |}]
;;
