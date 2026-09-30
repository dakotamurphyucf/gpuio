open Core

let gradient_pnm =
  let bytes = Buffer.create ((96 * 48 * 3) + 32) in
  Buffer.add_string bytes "P6\n96 48\n255\n";
  for y = 0 to 47 do
    for x = 0 to 95 do
      List.iter
        [ 80 + (x * 150 / 95); 120 + (y * 100 / 47); 210 - (x * 90 / 95) ]
        ~f:(fun value -> Buffer.add_char bytes (Char.of_int_exn value))
    done
  done;
  Buffer.contents bytes
;;
