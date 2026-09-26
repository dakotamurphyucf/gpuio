open Core

let () =
  let flag name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  if flag "--outline"
  then
    Outline_demo.run
      ~self_test:(flag "--self-test")
      ~gesture_test:(flag "--gesture-self-test")
  else Filesystem_demo.run ~self_test:(flag "--self-test")
;;
