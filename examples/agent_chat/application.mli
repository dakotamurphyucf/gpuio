(** Application-scoped conversations and per-window resources. The Workspace
    component owns Bonsai composition; this module supplies Eio capabilities. *)
val run
  :  self_test:bool
  -> native_test:bool
  -> workload_metrics:bool
  -> attachment_directory:Gpuio.File_path.t option
  -> motion:Gpuio.Animation.Preference.t
  -> unit
