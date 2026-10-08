open Core

let () =
  let flag name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  Gpuio_lifecycle_workload.run
    ~smoke:(flag "--smoke")
    ~background:(flag "--background")
    ~native_entities:false
    ~metal_memory:false
    ~presentation:false
;;
