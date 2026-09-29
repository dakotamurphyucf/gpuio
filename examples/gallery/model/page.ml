open Core

type t =
  | Presentation
  | Styles
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
  | Highlighting
  | Canvas
  | Assets
  | Charts
  | Motion
  | Extensions
  | Input
  | Observations
  | Responsive
  | Desktop
  | Runtime
[@@deriving equal, compare, sexp_of]

let all =
  [ Presentation
  ; Styles
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
  ; Highlighting
  ; Canvas
  ; Assets
  ; Charts
  ; Motion
  ; Extensions
  ; Input
  ; Observations
  ; Responsive
  ; Desktop
  ; Runtime
  ]
;;

let title = function
  | Presentation -> "Presentation"
  | Styles -> "Styling details"
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
  | Highlighting -> "Find & highlight"
  | Canvas -> "Canvas & drawing"
  | Assets -> "Images & icons"
  | Charts -> "Charts & data"
  | Motion -> "Motion & rhythm"
  | Extensions -> "Native extensions"
  | Input -> "Input & transfers"
  | Observations -> "Input observations"
  | Responsive -> "Responsive layouts"
  | Desktop -> "Desktop services"
  | Runtime -> "Runtime & windows"
;;

let description = function
  | Presentation -> "Expressive building blocks for thoughtful interfaces."
  | Styles -> "Small choices that make the interface feel considered."
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
  | Highlighting -> "Find the useful details, from a sentence to a growing notebook."
  | Canvas -> "A native drawing surface, with room to move."
  | Assets -> "Scalable artwork, crisp icons and thoughtful fallbacks."
  | Charts -> "Clear pictures, with the original values always within reach."
  | Motion -> "Thoughtful transitions, natural springs and a shared rhythm."
  | Extensions -> "Bring your own native components to the workspace."
  | Input -> "Direct gestures, simple transfers and keyboard alternatives."
  | Observations -> "Explore the details of everyday interactions."
  | Responsive -> "Thoughtful layouts that adapt to the space around them."
  | Desktop -> "A workspace that feels at home on your desktop."
  | Runtime -> "Native window behavior and visible resource ownership."
;;

let key = function
  | Presentation -> "presentation"
  | Styles -> "styles"
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
  | Highlighting -> "highlighting"
  | Canvas -> "canvas"
  | Assets -> "assets"
  | Charts -> "charts"
  | Motion -> "motion"
  | Extensions -> "extensions"
  | Input -> "input"
  | Observations -> "observations"
  | Responsive -> "responsive"
  | Desktop -> "desktop"
  | Runtime -> "runtime"
;;
