open Core

type t =
  | Presentation
  | Controls
  | Text_inputs
  | Numeric_inputs
  | Pickers
  | Overlays
  | Navigation
  | Feedback
  | Journeys
  | Collections
  | Documents
  | Canvas
  | Assets
  | Charts
  | Motion
  | Runtime
[@@deriving equal, compare, sexp_of]

let all =
  [ Presentation
  ; Controls
  ; Text_inputs
  ; Numeric_inputs
  ; Pickers
  ; Overlays
  ; Navigation
  ; Feedback
  ; Journeys
  ; Collections
  ; Documents
  ; Canvas
  ; Assets
  ; Charts
  ; Motion
  ; Runtime
  ]
;;

let title = function
  | Presentation -> "Presentation"
  | Controls -> "Selection & actions"
  | Text_inputs -> "Text editing"
  | Numeric_inputs -> "Numbers & codes"
  | Pickers -> "Dates & colors"
  | Overlays -> "Overlays & help"
  | Navigation -> "Navigation & layout"
  | Feedback -> "Commands & feedback"
  | Journeys -> "Carousels & journeys"
  | Collections -> "Lists, trees & tables"
  | Documents -> "Markdown & code"
  | Canvas -> "Canvas & drawing"
  | Assets -> "Images & icons"
  | Charts -> "Charts & data"
  | Motion -> "Motion & rhythm"
  | Runtime -> "Runtime & windows"
;;

let description = function
  | Presentation -> "Expressive building blocks for thoughtful interfaces."
  | Controls -> "Native controls that respond to pointer, keyboard and assistive tools."
  | Text_inputs -> "Native editing, Unicode, selection and composition."
  | Numeric_inputs -> "Precise quantities, ranges, ratings and verification codes."
  | Pickers -> "Explore a draft, then confirm the value that matters."
  | Overlays -> "Focused decisions and helpful context, without losing your place."
  | Navigation -> "Keep your place as the workspace grows around you."
  | Feedback -> "Actions within reach, and clear signals along the way."
  | Journeys -> "Native transitions with a sense of continuity."
  | Collections -> "Explore more while keeping the visible work small."
  | Documents -> "Rich documents, precise code and thoughtful change reviews."
  | Canvas -> "A native drawing surface, with room to move."
  | Assets -> "Scalable artwork, crisp icons and thoughtful fallbacks."
  | Charts -> "Clear pictures, with the original values always within reach."
  | Motion -> "Thoughtful transitions, natural springs and a shared rhythm."
  | Runtime -> "Native window behavior and visible resource ownership."
;;

let key = function
  | Presentation -> "presentation"
  | Controls -> "controls"
  | Text_inputs -> "text-inputs"
  | Numeric_inputs -> "numeric-inputs"
  | Pickers -> "pickers"
  | Overlays -> "overlays"
  | Navigation -> "navigation"
  | Feedback -> "feedback"
  | Journeys -> "journeys"
  | Collections -> "collections"
  | Documents -> "documents"
  | Canvas -> "canvas"
  | Assets -> "assets"
  | Charts -> "charts"
  | Motion -> "motion"
  | Runtime -> "runtime"
;;
