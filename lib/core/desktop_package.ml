open Core

type t =
  { file_name : string
  ; contents : string
  }

let file_name t = t.file_name
let contents t = t.contents

let printable_ascii s =
  String.for_all s ~f:(fun c -> Char.to_int c >= 32 && Char.to_int c <= 126)
;;

let desktop_string s =
  String.concat_map s ~f:(function
    | '\\' -> "\\\\"
    | ' ' -> "\\s"
    | c -> String.of_char c)
;;

let exec_argument s =
  let quoted =
    String.concat_map s ~f:(function
      | '%' -> "%%"
      | ('"' | '`' | '$' | '\\') as c -> "\\" ^ String.of_char c
      | c -> String.of_char c)
  in
  (* String unescaping precedes Exec quote unescaping. Spaces stay quoted. *)
  "\""
  ^ String.concat_map quoted ~f:(function
    | '\\' -> "\\\\"
    | c -> String.of_char c)
  ^ "\""
;;

let linux_entry identity ~executable ?(arguments = []) () =
  let executable = File_path.to_string executable in
  if (not (printable_ascii executable)) || String.contains executable '='
  then Or_error.error_string "desktop executable requires printable ASCII without '='"
  else if
    List.length arguments > 32
    || not
         (List.for_all arguments ~f:(fun arg ->
            String.length arg <= 1024 && printable_ascii arg))
  then
    Or_error.error_string
      "desktop arguments require at most 32 printable ASCII strings of 1024 bytes"
  else (
    let identifier = Desktop.Identity.identifier identity in
    let command =
      List.map (executable :: arguments) ~f:exec_argument |> String.concat ~sep:" "
    in
    let schemes =
      Desktop.Identity.schemes identity
      |> List.map ~f:(fun scheme ->
        "x-scheme-handler/" ^ Deep_link.Scheme.to_string scheme ^ ";")
      |> String.concat
    in
    Ok
      { file_name = identifier ^ ".desktop"
      ; contents =
          String.concat
            ~sep:"\n"
            ([ "[Desktop Entry]"
             ; "Type=Application"
             ; "Version=1.0"
             ; "Name=" ^ desktop_string (Desktop.Identity.name identity)
             ; "Exec=" ^ command ^ " --open-uris %U"
             ; "Terminal=false"
             ; "DBusActivatable=false"
             ]
             @ (if String.is_empty schemes then [] else [ "MimeType=" ^ schemes ])
             @ [ "" ])
      })
;;

let xml_string s =
  String.concat_map s ~f:(function
    | '&' -> "&amp;"
    | '<' -> "&lt;"
    | '>' -> "&gt;"
    | '"' -> "&quot;"
    | '\'' -> "&apos;"
    | c -> String.of_char c)
;;

let numeric_version value ~minimum ~maximum =
  let parts = String.split value ~on:'.' in
  List.length parts >= minimum
  && List.length parts <= maximum
  && List.for_all parts ~f:(fun part ->
    (not (String.is_empty part))
    && String.length part <= 4
    && String.for_all part ~f:Char.is_digit)
;;

let macos_info_plist identity ~executable ~version ~build =
  let portable =
    (not (String.is_empty executable))
    && String.length executable <= 128
    && (not (String.equal executable "." || String.equal executable ".."))
    && String.for_all executable ~f:(fun c ->
      Char.is_alpha c || Char.is_digit c || String.contains "._-" c)
  in
  let name = Desktop.Identity.name identity in
  if not portable
  then Or_error.error_string "bundle executable requires a portable basename"
  else if
    not
      (numeric_version version ~minimum:3 ~maximum:3
       && numeric_version build ~minimum:1 ~maximum:3)
  then Or_error.error_string "bundle version/build require bounded numeric components"
  else if
    String.is_substring name ~substring:"\239\191\190"
    || String.is_substring name ~substring:"\239\191\191"
  then Or_error.error_string "bundle name contains a character excluded by XML 1.0"
  else (
    let entry key value =
      "<key>" ^ key ^ "</key><string>" ^ xml_string value ^ "</string>"
    in
    let schemes = Desktop.Identity.schemes identity in
    let urls =
      if List.is_empty schemes
      then []
      else
        [ "<key>CFBundleURLTypes</key><array><dict>"
        ; entry "CFBundleURLName" (Desktop.Identity.identifier identity)
        ; entry "CFBundleTypeRole" "Viewer"
        ; "<key>CFBundleURLSchemes</key><array>"
        ]
        @ List.map schemes ~f:(fun scheme ->
          "<string>" ^ xml_string (Deep_link.Scheme.to_string scheme) ^ "</string>")
        @ [ "</array></dict></array>" ]
    in
    Ok
      { file_name = "Info.plist"
      ; contents =
          String.concat
            ~sep:"\n"
            ([ "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
             ; "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
                \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">"
             ; "<plist version=\"1.0\"><dict>"
             ; entry "CFBundleExecutable" executable
             ; entry "CFBundleIdentifier" (Desktop.Identity.identifier identity)
             ; entry "CFBundleName" name
             ; entry "CFBundlePackageType" "APPL"
             ; entry "CFBundleVersion" build
             ; entry "CFBundleShortVersionString" version
             ; entry "LSMinimumSystemVersion" "14.4"
             ; "<key>NSHighResolutionCapable</key><true/>"
             ]
             @ urls
             @ [ "</dict></plist>"; "" ])
      })
;;
