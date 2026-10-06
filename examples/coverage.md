# Example walkthrough coverage

Generated from [coverage.json](coverage.json). Read the [review guide](coverage-guide.md) before changing status.

**Pending means incomplete review**, even where a README exists. A reviewed row is documentation coverage, not platform acceptance.

## agent_chat

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [application.ml](agent_chat/application.ml), [application.mli](agent_chat/application.mli) | application-entry | [application.md](agent_chat/application.md) | reviewed |
| [main.ml](agent_chat/main.ml) | application-entry | [main.md](agent_chat/main.md) | reviewed |
| [fake_backend.ml](agent_chat/model/fake_backend.ml), [fake_backend.mli](agent_chat/model/fake_backend.mli) | support | [fake_backend.md](agent_chat/model/fake_backend.md) | reviewed |
| [fake_backend_test.ml](agent_chat/model/fake_backend_test.ml) | test-support | [fake_backend.md](agent_chat/model/fake_backend.md) | reviewed |
| [annotation_settings.ml](agent_chat/runtime/annotation_settings.ml), [annotation_settings.mli](agent_chat/runtime/annotation_settings.mli) | component | [annotation_settings.md](agent_chat/runtime/annotation_settings.md) | reviewed |
| [artifact_sidebar.ml](agent_chat/runtime/artifact_sidebar.ml), [artifact_sidebar.mli](agent_chat/runtime/artifact_sidebar.mli) | component | [artifact_sidebar.md](agent_chat/runtime/artifact_sidebar.md) | reviewed |
| [artifact_tour.ml](agent_chat/runtime/artifact_tour.ml), [artifact_tour.mli](agent_chat/runtime/artifact_tour.mli) | component | [artifact_tour.md](agent_chat/runtime/artifact_tour.md) | reviewed |
| [chat_message.ml](agent_chat/runtime/chat_message.ml), [chat_message.mli](agent_chat/runtime/chat_message.mli) | component | [chat_message.md](agent_chat/runtime/chat_message.md) | reviewed |
| [chat_motion.ml](agent_chat/runtime/chat_motion.ml), [chat_motion.mli](agent_chat/runtime/chat_motion.mli) | component | [chat_motion.md](agent_chat/runtime/chat_motion.md) | reviewed |
| [contributor_portrait.ml](agent_chat/runtime/contributor_portrait.ml), [contributor_portrait.mli](agent_chat/runtime/contributor_portrait.mli) | component | [contributor_portrait.md](agent_chat/runtime/contributor_portrait.md) | reviewed |
| [conversation.ml](agent_chat/runtime/conversation.ml), [conversation.mli](agent_chat/runtime/conversation.mli) | component | [conversation.md](agent_chat/runtime/conversation.md) | reviewed |
| [diagram.ml](agent_chat/runtime/diagram.ml), [diagram.mli](agent_chat/runtime/diagram.mli) | component | [diagram.md](agent_chat/runtime/diagram.md) | reviewed |
| [fixture_job.ml](agent_chat/runtime/fixture_job.ml), [fixture_job.mli](agent_chat/runtime/fixture_job.mli) | component | [fixture_job.md](agent_chat/runtime/fixture_job.md) | reviewed |
| [generation_settings.ml](agent_chat/runtime/generation_settings.ml), [generation_settings.mli](agent_chat/runtime/generation_settings.mli) | component | [generation_settings.md](agent_chat/runtime/generation_settings.md) | reviewed |
| [icons.ml](agent_chat/runtime/icons.ml), [icons.mli](agent_chat/runtime/icons.mli) | support | [icons.md](agent_chat/runtime/icons.md) | reviewed |
| [inspector.ml](agent_chat/runtime/inspector.ml), [inspector.mli](agent_chat/runtime/inspector.mli) | component | [inspector.md](agent_chat/runtime/inspector.md) | reviewed |
| [palette.ml](agent_chat/runtime/palette.ml), [palette.mli](agent_chat/runtime/palette.mli) | support | [palette.md](agent_chat/runtime/palette.md) | reviewed |
| [query_loading.ml](agent_chat/runtime/query_loading.ml), [query_loading.mli](agent_chat/runtime/query_loading.mli) | component | [query_loading.md](agent_chat/runtime/query_loading.md) | reviewed |
| [responsive.ml](agent_chat/runtime/responsive.ml), [responsive.mli](agent_chat/runtime/responsive.mli) | component | [responsive.md](agent_chat/runtime/responsive.md) | reviewed |
| [result_actions.ml](agent_chat/runtime/result_actions.ml), [result_actions.mli](agent_chat/runtime/result_actions.mli) | component | [result_actions.md](agent_chat/runtime/result_actions.md) | reviewed |
| [result_data.ml](agent_chat/runtime/result_data.ml), [result_data.mli](agent_chat/runtime/result_data.mli) | component | [result_data.md](agent_chat/runtime/result_data.md) | reviewed |
| [results.ml](agent_chat/runtime/results.ml), [results.mli](agent_chat/runtime/results.mli) | component | [results.md](agent_chat/runtime/results.md) | reviewed |
| [review.ml](agent_chat/runtime/review.ml), [review.mli](agent_chat/runtime/review.mli) | component | [review.md](agent_chat/runtime/review.md) | reviewed |
| [review_feedback.ml](agent_chat/runtime/review_feedback.ml), [review_feedback.mli](agent_chat/runtime/review_feedback.mli) | component | [review_feedback.md](agent_chat/runtime/review_feedback.md) | reviewed |
| [run_diagram.ml](agent_chat/runtime/run_diagram.ml), [run_diagram.mli](agent_chat/runtime/run_diagram.mli) | component | [run_diagram.md](agent_chat/runtime/run_diagram.md) | reviewed |
| [schedule_data.ml](agent_chat/runtime/schedule_data.ml), [schedule_data.mli](agent_chat/runtime/schedule_data.mli) | component | [schedule_data.md](agent_chat/runtime/schedule_data.md) | reviewed |
| [schedule_settings.ml](agent_chat/runtime/schedule_settings.ml), [schedule_settings.mli](agent_chat/runtime/schedule_settings.mli) | component | [schedule_settings.md](agent_chat/runtime/schedule_settings.md) | reviewed |
| [score_range.ml](agent_chat/runtime/score_range.ml), [score_range.mli](agent_chat/runtime/score_range.mli) | component | [score_range.md](agent_chat/runtime/score_range.md) | reviewed |
| [settings.ml](agent_chat/runtime/settings.ml), [settings.mli](agent_chat/runtime/settings.mli) | component | [settings.md](agent_chat/runtime/settings.md) | reviewed |
| [source_data.ml](agent_chat/runtime/source_data.ml), [source_data.mli](agent_chat/runtime/source_data.mli) | component | [source_data.md](agent_chat/runtime/source_data.md) | reviewed |
| [sources.ml](agent_chat/runtime/sources.ml), [sources.mli](agent_chat/runtime/sources.mli) | component | [sources.md](agent_chat/runtime/sources.md) | reviewed |
| [workspace.ml](agent_chat/runtime/workspace.ml), [workspace.mli](agent_chat/runtime/workspace.mli) | component | [workspace.md](agent_chat/runtime/workspace.md) | reviewed |
| [self_test.ml](agent_chat/self_test.ml), [self_test.mli](agent_chat/self_test.mli) | test-support | [self_test.md](agent_chat/self_test.md) | reviewed |
| [workload_metrics.ml](agent_chat/workload_metrics.ml), [workload_metrics.mli](agent_chat/workload_metrics.mli) | component | [workload_metrics.md](agent_chat/workload_metrics.md) | reviewed |
## animation

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](animation/main.ml) | application-entry | [main.md](animation/main.md) | reviewed |
## animation_program

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](animation_program/main.ml) | application-entry | [main.md](animation_program/main.md) | reviewed |
## asset_upload

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](asset_upload/main.ml) | diagnostic | [main.md](asset_upload/main.md) | reviewed |
## bridge

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](bridge/main.ml) | diagnostic | [main.md](bridge/main.md) | reviewed |
## calendar

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](calendar/main.ml) | application-entry | [main.md](calendar/main.md) | reviewed |
| [picker.ml](calendar/picker.ml) | component | [picker.md](calendar/picker.md) | reviewed |
## canvas

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](canvas/main.ml) | application-entry | [main.md](canvas/main.md) | reviewed |
| [plot.ml](canvas/plot.ml), [plot.mli](canvas/plot.mli) | component | [plot.md](canvas/plot.md) | reviewed |
## canvas_upload

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](canvas_upload/main.ml) | diagnostic | [main.md](canvas_upload/main.md) | reviewed |
## chart_stream

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](chart_stream/main.ml) | application-entry | [main.md](chart_stream/main.md) | reviewed |
## chart_upload

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](chart_upload/main.ml) | diagnostic | [main.md](chart_upload/main.md) | reviewed |
## charts

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [gallery.ml](charts/gallery.ml) | component | [gallery.md](charts/gallery.md) | reviewed |
| [main.ml](charts/main.ml) | application-entry | [main.md](charts/main.md) | reviewed |
| [categorical.ml](charts/samples/categorical.ml), [categorical.mli](charts/samples/categorical.mli) | component | [categorical.md](charts/samples/categorical.md) | reviewed |
| [gpuio_chart_samples.ml](charts/samples/gpuio_chart_samples.ml), [gpuio_chart_samples.mli](charts/samples/gpuio_chart_samples.mli) | support | [gpuio_chart_samples.md](charts/samples/gpuio_chart_samples.md) | reviewed |
| [inspection.ml](charts/samples/inspection.ml), [inspection.mli](charts/samples/inspection.mli) | component | [inspection.md](charts/samples/inspection.md) | reviewed |
| [ordinal_colors.ml](charts/samples/ordinal_colors.ml), [ordinal_colors.mli](charts/samples/ordinal_colors.mli) | component | [ordinal_colors.md](charts/samples/ordinal_colors.md) | reviewed |
| [sankey_presentation.ml](charts/samples/sankey_presentation.ml), [sankey_presentation.mli](charts/samples/sankey_presentation.mli) | component | [sankey_presentation.md](charts/samples/sankey_presentation.md) | reviewed |
| [stacked.ml](charts/samples/stacked.ml), [stacked.mli](charts/samples/stacked.mli) | component | [stacked.md](charts/samples/stacked.md) | reviewed |
## color_input

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](color_input/main.ml) | application-entry | [main.md](color_input/main.md) | reviewed |
| [picker.ml](color_input/picker.ml) | component | [picker.md](color_input/picker.md) | reviewed |
## combobox

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](combobox/main.ml) | application-entry | [main.md](combobox/main.md) | reviewed |
## commands

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](commands/main.ml) | application-entry | [main.md](commands/main.md) | reviewed |
## container_query

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](container_query/main.ml) | application-entry | [main.md](container_query/main.md) | reviewed |
## controls

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](controls/main.ml) | application-entry | [main.md](controls/main.md) | reviewed |
## desktop

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](desktop/main.ml) | application-entry | [main.md](desktop/main.md) | reviewed |
## document_profile_package

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [gpuio_example_document.ml](document_profile_package/ocaml/gpuio_example_document.ml), [gpuio_example_document.mli](document_profile_package/ocaml/gpuio_example_document.mli) | extension-author | [gpuio_example_document.md](document_profile_package/ocaml/gpuio_example_document.md) | reviewed |
| [lib.rs](document_profile_package/rust/src/lib.rs) | extension-author | [lib.md](document_profile_package/rust/src/lib.md) | reviewed |
| [scroll_card.rs](document_profile_package/rust/src/scroll_card.rs) | extension-author | [scroll_card.md](document_profile_package/rust/src/scroll_card.md) | reviewed |
| [document_profile_example_test.ml](document_profile_package/test/document_profile_example_test.ml) | test-support | [document_profile_example_test.md](document_profile_package/test/document_profile_example_test.md) | reviewed |
## documents

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](documents/main.ml) | application-entry | [main.md](documents/main.md) | reviewed |
## drag_drop

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](drag_drop/main.ml) | application-entry | [main.md](drag_drop/main.md) | reviewed |
## drag_drop_desktop

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](drag_drop_desktop/main.ml) | application-entry | [main.md](drag_drop_desktop/main.md) | reviewed |
## extension_consumer

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [backend.ml](extension_consumer/backend/backend.ml) | generated-registration | [backend.md](extension_consumer/backend/backend.md) | reviewed |
| [registration.rs](extension_consumer/backend/registration.rs) | generated-registration | [registration.md](extension_consumer/backend/registration.md) | reviewed |
| [main.ml](extension_consumer/main.ml) | application-entry | [main.md](extension_consumer/main.md) | reviewed |
## extension_package

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [gpuio_example_counter.ml](extension_package/ocaml/gpuio_example_counter.ml), [gpuio_example_counter.mli](extension_package/ocaml/gpuio_example_counter.mli) | extension-author | [gpuio_example_counter.md](extension_package/ocaml/gpuio_example_counter.md) | reviewed |
| [lib.rs](extension_package/rust/src/lib.rs) | extension-author | [lib.md](extension_package/rust/src/lib.md) | reviewed |
## file_dialogs

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](file_dialogs/main.ml) | application-entry | [main.md](file_dialogs/main.md) | reviewed |
## foundation

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](foundation/main.ml) | historical-bootstrap | [main.md](foundation/main.md) | reviewed |
| [native.ml](foundation/native.ml) | historical-bootstrap | [native.md](foundation/native.md) | reviewed |
| [wire.ml](foundation/wire.ml) | historical-bootstrap | [wire.md](foundation/wire.md) | reviewed |
## gallery

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [alert_preview.ml](gallery/alert_preview.ml), [alert_preview.mli](gallery/alert_preview.mli) | component | [alert_preview.md](gallery/alert_preview.md) | reviewed |
| [application.ml](gallery/application.ml), [application.mli](gallery/application.mli) | application-entry | [application.md](gallery/application.md) | reviewed |
| [aspect_preview.ml](gallery/aspect_preview.ml) | component | [aspect_preview.md](gallery/aspect_preview.md) | reviewed |
| [assets_page.ml](gallery/assets_page.ml), [assets_page.mli](gallery/assets_page.mli) | component | [assets_page.md](gallery/assets_page.md) | reviewed |
| [attachment_preview.ml](gallery/attachment_preview.ml) | component | [attachment_preview.md](gallery/attachment_preview.md) | reviewed |
| [avatar_preview.ml](gallery/avatar_preview.ml), [avatar_preview.mli](gallery/avatar_preview.mli) | component | [avatar_preview.md](gallery/avatar_preview.md) | reviewed |
| [badge_preview.ml](gallery/badge_preview.ml), [badge_preview.mli](gallery/badge_preview.mli) | component | [badge_preview.md](gallery/badge_preview.md) | reviewed |
| [binding_preview.ml](gallery/binding_preview.ml), [binding_preview.mli](gallery/binding_preview.mli) | component | [binding_preview.md](gallery/binding_preview.md) | reviewed |
| [button_appearance_preview.ml](gallery/button_appearance_preview.ml), [button_appearance_preview.mli](gallery/button_appearance_preview.mli) | component | [button_appearance_preview.md](gallery/button_appearance_preview.md) | reviewed |
| [button_preview.ml](gallery/button_preview.ml), [button_preview.mli](gallery/button_preview.mli) | component | [button_preview.md](gallery/button_preview.md) | reviewed |
| [canvas_page.ml](gallery/canvas_page.ml), [canvas_page.mli](gallery/canvas_page.mli) | component | [canvas_page.md](gallery/canvas_page.md) | reviewed |
| [carousel_track_preview.ml](gallery/carousel_track_preview.ml), [carousel_track_preview.mli](gallery/carousel_track_preview.mli) | component | [carousel_track_preview.md](gallery/carousel_track_preview.md) | reviewed |
| [charts_page.ml](gallery/charts_page.ml), [charts_page.mli](gallery/charts_page.mli) | component | [charts_page.md](gallery/charts_page.md) | reviewed |
| [chat_composition_preview.ml](gallery/chat_composition_preview.ml), [chat_composition_preview.mli](gallery/chat_composition_preview.mli) | component | [chat_composition_preview.md](gallery/chat_composition_preview.md) | reviewed |
| [chat_list_preview.ml](gallery/chat_list_preview.ml), [chat_list_preview.mli](gallery/chat_list_preview.mli) | component | [chat_list_preview.md](gallery/chat_list_preview.md) | reviewed |
| [checkable_navigation_preview.ml](gallery/checkable_navigation_preview.ml), [checkable_navigation_preview.mli](gallery/checkable_navigation_preview.mli) | component | [checkable_navigation_preview.md](gallery/checkable_navigation_preview.md) | reviewed |
| [choice_picker_cases.ml](gallery/choice_picker_cases.ml), [choice_picker_cases.mli](gallery/choice_picker_cases.mli) | component | [choice_picker_cases.md](gallery/choice_picker_cases.md) | reviewed |
| [choice_picker_preview.ml](gallery/choice_picker_preview.ml), [choice_picker_preview.mli](gallery/choice_picker_preview.mli) | component | [choice_picker_preview.md](gallery/choice_picker_preview.md) | reviewed |
| [collections_page.ml](gallery/collections_page.ml), [collections_page.mli](gallery/collections_page.mli) | component | [collections_page.md](gallery/collections_page.md) | reviewed |
| [command_tooltip_preview.ml](gallery/command_tooltip_preview.ml) | component | [command_tooltip_preview.md](gallery/command_tooltip_preview.md) | reviewed |
| [component.ml](gallery/component.ml), [component.mli](gallery/component.mli) | component | [component.md](gallery/component.md) | reviewed |
| [content_hint_preview.ml](gallery/content_hint_preview.ml), [content_hint_preview.mli](gallery/content_hint_preview.mli) | component | [content_hint_preview.md](gallery/content_hint_preview.md) | reviewed |
| [control_appearance_preview.ml](gallery/control_appearance_preview.ml), [control_appearance_preview.mli](gallery/control_appearance_preview.mli) | component | [control_appearance_preview.md](gallery/control_appearance_preview.md) | reviewed |
| [description_preview.ml](gallery/description_preview.ml), [description_preview.mli](gallery/description_preview.mli) | component | [description_preview.md](gallery/description_preview.md) | reviewed |
| [desktop_page.ml](gallery/desktop_page.ml), [desktop_page.mli](gallery/desktop_page.mli) | component | [desktop_page.md](gallery/desktop_page.md) | reviewed |
| [desktop_session.ml](gallery/desktop_session.ml), [desktop_session.mli](gallery/desktop_session.mli) | component | [desktop_session.md](gallery/desktop_session.md) | reviewed |
| [disclosure_preview.ml](gallery/disclosure_preview.ml) | component | [disclosure_preview.md](gallery/disclosure_preview.md) | reviewed |
| [documents_page.ml](gallery/documents_page.ml), [documents_page.mli](gallery/documents_page.mli) | component | [documents_page.md](gallery/documents_page.md) | reviewed |
| [edit_filter_preview.ml](gallery/edit_filter_preview.ml), [edit_filter_preview.mli](gallery/edit_filter_preview.mli) | component | [edit_filter_preview.md](gallery/edit_filter_preview.md) | reviewed |
| [embedded_palette_preview.ml](gallery/embedded_palette_preview.ml), [embedded_palette_preview.mli](gallery/embedded_palette_preview.mli) | component | [embedded_palette_preview.md](gallery/embedded_palette_preview.md) | reviewed |
| [empty_preview.ml](gallery/empty_preview.ml), [empty_preview.mli](gallery/empty_preview.mli) | component | [empty_preview.md](gallery/empty_preview.md) | reviewed |
| [extensions_page.ml](gallery/extensions_page.ml), [extensions_page.mli](gallery/extensions_page.mli) | component | [extensions_page.md](gallery/extensions_page.md) | reviewed |
| [external_palette_preview.ml](gallery/external_palette_preview.ml), [external_palette_preview.mli](gallery/external_palette_preview.mli) | component | [external_palette_preview.md](gallery/external_palette_preview.md) | reviewed |
| [feedback_page.ml](gallery/feedback_page.ml), [feedback_page.mli](gallery/feedback_page.mli) | component | [feedback_page.md](gallery/feedback_page.md) | reviewed |
| [settings_file.ml](gallery/files/settings_file.ml), [settings_file.mli](gallery/files/settings_file.mli) | component | [settings_file.md](gallery/files/settings_file.md) | reviewed |
| [settings_file_test.ml](gallery/files/test/settings_file_test.ml) | test-support | [settings_file.md](gallery/files/settings_file.md) | reviewed |
| [theme_file_test.ml](gallery/files/test/theme_file_test.ml) | test-support | [theme_file.md](gallery/files/theme_file.md) | reviewed |
| [theme_file.ml](gallery/files/theme_file.ml), [theme_file.mli](gallery/files/theme_file.mli) | component | [theme_file.md](gallery/files/theme_file.md) | reviewed |
| [form_preview.ml](gallery/form_preview.ml), [form_preview.mli](gallery/form_preview.mli) | component | [form_preview.md](gallery/form_preview.md) | reviewed |
| [format_preview.ml](gallery/format_preview.ml), [format_preview.mli](gallery/format_preview.mli) | component | [format_preview.md](gallery/format_preview.md) | reviewed |
| [group_preview.ml](gallery/group_preview.ml), [group_preview.mli](gallery/group_preview.mli) | component | [group_preview.md](gallery/group_preview.md) | reviewed |
| [highlight_page.ml](gallery/highlight_page.ml), [highlight_page.mli](gallery/highlight_page.mli) | component | [highlight_page.md](gallery/highlight_page.md) | reviewed |
| [horizontal_list_preview.ml](gallery/horizontal_list_preview.ml), [horizontal_list_preview.mli](gallery/horizontal_list_preview.mli) | component | [horizontal_list_preview.md](gallery/horizontal_list_preview.md) | reviewed |
| [image_samples.ml](gallery/image_samples.ml), [image_samples.mli](gallery/image_samples.mli) | component | [image_samples.md](gallery/image_samples.md) | reviewed |
| [input_page.ml](gallery/input_page.ml), [input_page.mli](gallery/input_page.mli) | component | [input_page.md](gallery/input_page.md) | reviewed |
| [journeys_page.ml](gallery/journeys_page.ml), [journeys_page.mli](gallery/journeys_page.mli) | component | [journeys_page.md](gallery/journeys_page.md) | reviewed |
| [keyboard_preview.ml](gallery/keyboard_preview.ml), [keyboard_preview.mli](gallery/keyboard_preview.mli) | component | [keyboard_preview.md](gallery/keyboard_preview.md) | reviewed |
| [label_preview.ml](gallery/label_preview.ml), [label_preview.mli](gallery/label_preview.mli) | component | [label_preview.md](gallery/label_preview.md) | reviewed |
| [link_preview.ml](gallery/link_preview.ml), [link_preview.mli](gallery/link_preview.mli) | component | [link_preview.md](gallery/link_preview.md) | reviewed |
| [main.ml](gallery/main.ml) | application-entry | [application.md](gallery/application.md) | reviewed |
| [marker_preview.ml](gallery/marker_preview.ml), [marker_preview.mli](gallery/marker_preview.mli) | component | [marker_preview.md](gallery/marker_preview.md) | reviewed |
| [menu_preview.ml](gallery/menu_preview.ml), [menu_preview.mli](gallery/menu_preview.mli) | component | [menu_preview.md](gallery/menu_preview.md) | reviewed |
| [appearance.ml](gallery/model/appearance.ml), [appearance.mli](gallery/model/appearance.mli) | support | [appearance.md](gallery/model/appearance.md) | reviewed |
| [canvas_study.ml](gallery/model/canvas_study.ml), [canvas_study.mli](gallery/model/canvas_study.mli) | support | [canvas_study.md](gallery/model/canvas_study.md) | reviewed |
| [diff_state.ml](gallery/model/diff_state.ml), [diff_state.mli](gallery/model/diff_state.mli) | support | [diff_state.md](gallery/model/diff_state.md) | reviewed |
| [editor_visit.ml](gallery/model/editor_visit.ml), [editor_visit.mli](gallery/model/editor_visit.mli) | support | [editor_visit.md](gallery/model/editor_visit.md) | reviewed |
| [extension_state.ml](gallery/model/extension_state.ml), [extension_state.mli](gallery/model/extension_state.mli) | support | [extension_state.md](gallery/model/extension_state.md) | reviewed |
| [feedback_state.ml](gallery/model/feedback_state.ml), [feedback_state.mli](gallery/model/feedback_state.mli) | support | [feedback_state.md](gallery/model/feedback_state.md) | reviewed |
| [message_follow.ml](gallery/model/message_follow.ml), [message_follow.mli](gallery/model/message_follow.mli) | support | [message_follow.md](gallery/model/message_follow.md) | reviewed |
| [message_stream.ml](gallery/model/message_stream.ml), [message_stream.mli](gallery/model/message_stream.mli) | support | [message_stream.md](gallery/model/message_stream.md) | reviewed |
| [numeric_state.ml](gallery/model/numeric_state.ml), [numeric_state.mli](gallery/model/numeric_state.mli) | support | [numeric_state.md](gallery/model/numeric_state.md) | reviewed |
| [page.ml](gallery/model/page.ml), [page.mli](gallery/model/page.mli) | support | [page.md](gallery/model/page.md) | reviewed |
| [picker_state.ml](gallery/model/picker_state.ml), [picker_state.mli](gallery/model/picker_state.mli) | support | [picker_state.md](gallery/model/picker_state.md) | reviewed |
| [selection_state.ml](gallery/model/selection_state.ml), [selection_state.mli](gallery/model/selection_state.mli) | support | [selection_state.md](gallery/model/selection_state.md) | reviewed |
| [settings_state.ml](gallery/model/settings_state.ml), [settings_state.mli](gallery/model/settings_state.mli) | support | [settings_state.md](gallery/model/settings_state.md) | reviewed |
| [theme_profile.ml](gallery/model/theme_profile.ml), [theme_profile.mli](gallery/model/theme_profile.mli) | support | [theme_profile.md](gallery/model/theme_profile.md) | reviewed |
| [theme_selection.ml](gallery/model/theme_selection.ml), [theme_selection.mli](gallery/model/theme_selection.mli) | support | [theme_selection.md](gallery/model/theme_selection.md) | reviewed |
| [motion_page.ml](gallery/motion_page.ml), [motion_page.mli](gallery/motion_page.mli) | component | [motion_page.md](gallery/motion_page.md) | reviewed |
| [navigation_page.ml](gallery/navigation_page.ml), [navigation_page.mli](gallery/navigation_page.mli) | component | [navigation_page.md](gallery/navigation_page.md) | reviewed |
| [number_preview.ml](gallery/number_preview.ml), [number_preview.mli](gallery/number_preview.mli) | component | [number_preview.md](gallery/number_preview.md) | reviewed |
| [numeric_page.ml](gallery/numeric_page.ml), [numeric_page.mli](gallery/numeric_page.mli) | component | [numeric_page.md](gallery/numeric_page.md) | reviewed |
| [observations_page.ml](gallery/observations_page.ml), [observations_page.mli](gallery/observations_page.mli) | component | [observations_page.md](gallery/observations_page.md) | reviewed |
| [overlays_page.ml](gallery/overlays_page.ml), [overlays_page.mli](gallery/overlays_page.mli) | component | [overlays_page.md](gallery/overlays_page.md) | reviewed |
| [pages.ml](gallery/pages.ml), [pages.mli](gallery/pages.mli) | support | [pages.md](gallery/pages.md) | reviewed |
| [pagination_preview.ml](gallery/pagination_preview.ml) | component | [pagination_preview.md](gallery/pagination_preview.md) | reviewed |
| [palette.ml](gallery/palette.ml), [palette.mli](gallery/palette.mli) | support | [palette.md](gallery/palette.md) | reviewed |
| [password_preview.ml](gallery/password_preview.ml), [password_preview.mli](gallery/password_preview.mli) | component | [password_preview.md](gallery/password_preview.md) | reviewed |
| [pickers_page.ml](gallery/pickers_page.ml), [pickers_page.mli](gallery/pickers_page.mli) | component | [pickers_page.md](gallery/pickers_page.md) | reviewed |
| [placement_preview.ml](gallery/placement_preview.ml) | component | [placement_preview.md](gallery/placement_preview.md) | reviewed |
| [preview_scope.ml](gallery/preview_scope.ml), [preview_scope.mli](gallery/preview_scope.mli) | component | [preview_scope.md](gallery/preview_scope.md) | reviewed |
| [progress_preview.ml](gallery/progress_preview.ml), [progress_preview.mli](gallery/progress_preview.mli) | component | [progress_preview.md](gallery/progress_preview.md) | reviewed |
| [responsive_page.ml](gallery/responsive_page.ml), [responsive_page.mli](gallery/responsive_page.mli) | component | [responsive_page.md](gallery/responsive_page.md) | reviewed |
| [runtime_page.ml](gallery/runtime_page.ml), [runtime_page.mli](gallery/runtime_page.mli) | component | [runtime_page.md](gallery/runtime_page.md) | reviewed |
| [scrollbar_preview.ml](gallery/scrollbar_preview.ml), [scrollbar_preview.mli](gallery/scrollbar_preview.mli) | component | [scrollbar_preview.md](gallery/scrollbar_preview.md) | reviewed |
| [selectable_preview.ml](gallery/selectable_preview.ml), [selectable_preview.mli](gallery/selectable_preview.mli) | component | [selectable_preview.md](gallery/selectable_preview.md) | reviewed |
| [selection_preview.ml](gallery/selection_preview.ml), [selection_preview.mli](gallery/selection_preview.mli) | component | [selection_preview.md](gallery/selection_preview.md) | reviewed |
| [separator_preview.ml](gallery/separator_preview.ml) | component | [separator_preview.md](gallery/separator_preview.md) | reviewed |
| [settings_preview.ml](gallery/settings_preview.ml), [settings_preview.mli](gallery/settings_preview.mli) | component | [settings_preview.md](gallery/settings_preview.md) | reviewed |
| [shell.ml](gallery/shell.ml), [shell.mli](gallery/shell.mli) | component | [shell.md](gallery/shell.md) | reviewed |
| [shimmer_preview.ml](gallery/shimmer_preview.ml), [shimmer_preview.mli](gallery/shimmer_preview.mli) | component | [shimmer_preview.md](gallery/shimmer_preview.md) | reviewed |
| [slider_preview.ml](gallery/slider_preview.ml) | component | [slider_preview.md](gallery/slider_preview.md) | reviewed |
| [spinner_preview.ml](gallery/spinner_preview.ml), [spinner_preview.mli](gallery/spinner_preview.mli) | component | [spinner_preview.md](gallery/spinner_preview.md) | reviewed |
| [split_group_preview.ml](gallery/split_group_preview.ml) | component | [split_group_preview.md](gallery/split_group_preview.md) | reviewed |
| [split_preview.ml](gallery/split_preview.ml), [split_preview.mli](gallery/split_preview.mli) | component | [split_preview.md](gallery/split_preview.md) | reviewed |
| [stepper_preview.ml](gallery/stepper_preview.ml), [stepper_preview.mli](gallery/stepper_preview.mli) | component | [stepper_preview.md](gallery/stepper_preview.md) | reviewed |
| [structural_table_preview.ml](gallery/structural_table_preview.ml), [structural_table_preview.mli](gallery/structural_table_preview.mli) | component | [structural_table_preview.md](gallery/structural_table_preview.md) | reviewed |
| [styles_page.ml](gallery/styles_page.ml), [styles_page.mli](gallery/styles_page.mli) | component | [styles_page.md](gallery/styles_page.md) | reviewed |
| [tab_content_preview.ml](gallery/tab_content_preview.ml), [tab_content_preview.mli](gallery/tab_content_preview.mli) | component | [tab_content_preview.md](gallery/tab_content_preview.md) | reviewed |
| [tag_preview.ml](gallery/tag_preview.ml), [tag_preview.mli](gallery/tag_preview.mli) | component | [tag_preview.md](gallery/tag_preview.md) | reviewed |
| [textarea_preview.ml](gallery/textarea_preview.ml), [textarea_preview.mli](gallery/textarea_preview.mli) | component | [textarea_preview.md](gallery/textarea_preview.md) | reviewed |
| [theme_preview.ml](gallery/theme_preview.ml), [theme_preview.mli](gallery/theme_preview.mli) | component | [theme_preview.md](gallery/theme_preview.md) | reviewed |
| [window_input_preview.ml](gallery/window_input_preview.ml) | component | [window_input_preview.md](gallery/window_input_preview.md) | reviewed |
| [window_selection_preview.ml](gallery/window_selection_preview.ml), [window_selection_preview.mli](gallery/window_selection_preview.mli) | component | [window_selection_preview.md](gallery/window_selection_preview.md) | reviewed |
## getting_started

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](getting_started/main.ml) | application-entry | [README.md](getting_started/README.md) | reviewed |
## images

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](images/main.ml) | application-entry | [main.md](images/main.md) | reviewed |
## lifecycle_workload

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [gpuio_lifecycle_workload.ml](lifecycle_workload/gpuio_lifecycle_workload.ml), [gpuio_lifecycle_workload.mli](lifecycle_workload/gpuio_lifecycle_workload.mli) | benchmark | [gpuio_lifecycle_workload.md](lifecycle_workload/gpuio_lifecycle_workload.md) | reviewed |
## menu_controller

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [component.ml](menu_controller/component.ml), [component.mli](menu_controller/component.mli) | component | [component.md](menu_controller/component.md) | reviewed |
| [main.ml](menu_controller/main.ml) | application-entry | [main.md](menu_controller/main.md) | reviewed |
| [multiwindow.ml](menu_controller/multiwindow.ml), [multiwindow.mli](menu_controller/multiwindow.mli) | component | [multiwindow.md](menu_controller/multiwindow.md) | reviewed |
## menus

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](menus/main.ml) | application-entry | [main.md](menus/main.md) | reviewed |
## navigation

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [carousel_lab.ml](navigation/carousel_lab.ml), [carousel_lab.mli](navigation/carousel_lab.mli) | component | [carousel_lab.md](navigation/carousel_lab.md) | reviewed |
| [main.ml](navigation/main.ml) | application-entry | [main.md](navigation/main.md) | reviewed |
| [sidebar_icons.ml](navigation/sidebar_icons.ml) | component | [sidebar_icons.md](navigation/sidebar_icons.md) | reviewed |
## notification

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](notification/main.ml) | application-entry | [main.md](notification/main.md) | reviewed |
## numeric

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](numeric/main.ml) | application-entry | [main.md](numeric/main.md) | reviewed |
| [number.ml](numeric/number.ml) | component | [number.md](numeric/number.md) | reviewed |
| [otp.ml](numeric/otp.ml) | component | [otp.md](numeric/otp.md) | reviewed |
## overlays

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](overlays/main.ml) | application-entry | [main.md](overlays/main.md) | reviewed |
## palette

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](palette/main.ml) | application-entry | [main.md](palette/main.md) | reviewed |
## performance

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [backend.ml](performance/backend/backend.ml) | generated-registration | [backend.md](performance/backend/backend.md) | reviewed |
| [registration.rs](performance/backend/registration.rs) | generated-registration | [registration.md](performance/backend/registration.md) | reviewed |
| [main.ml](performance/main.ml) | benchmark | [main.md](performance/main.md) | reviewed |
## performance_document

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](performance_document/main.ml) | benchmark | [main.md](performance_document/main.md) | reviewed |
## performance_idle

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](performance_idle/main.ml) | benchmark | [main.md](performance_idle/main.md) | reviewed |
## performance_lifecycle

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](performance_lifecycle/main.ml) | benchmark | [main.md](performance_lifecycle/main.md) | reviewed |
## performance_presented

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [backend.ml](performance_presented/backend/backend.ml) | generated-registration | [backend.md](performance_presented/backend/backend.md) | reviewed |
| [registration.rs](performance_presented/backend/registration.rs) | generated-registration | [registration.md](performance_presented/backend/registration.md) | reviewed |
## performance_probe

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [gpuio_performance_probe.ml](performance_probe/ocaml/gpuio_performance_probe.ml), [gpuio_performance_probe.mli](performance_probe/ocaml/gpuio_performance_probe.mli) | benchmark | [gpuio_performance_probe.md](performance_probe/ocaml/gpuio_performance_probe.md) | reviewed |
| [lib.rs](performance_probe/rust/src/lib.rs) | benchmark | [lib.md](performance_probe/rust/src/lib.md) | reviewed |
| [presentation.rs](performance_probe/rust/src/presentation.rs) | benchmark | [presentation.md](performance_probe/rust/src/presentation.md) | reviewed |
## performance_streaming

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](performance_streaming/main.ml) | benchmark | [main.md](performance_streaming/main.md) | reviewed |
| [stream_model.ml](performance_streaming/stream_model.ml), [stream_model.mli](performance_streaming/stream_model.mli) | benchmark | [stream_model.md](performance_streaming/stream_model.md) | reviewed |
## performance_table

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](performance_table/main.ml) | benchmark | [main.md](performance_table/main.md) | reviewed |
| [page_cache.ml](performance_table/page_cache.ml), [page_cache.mli](performance_table/page_cache.mli) | benchmark | [page_cache.md](performance_table/page_cache.md) | reviewed |
## pointer

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](pointer/main.ml) | application-entry | [main.md](pointer/main.md) | reviewed |
## presentation

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [avatar_assets.ml](presentation/avatar_assets.ml) | component | [avatar_assets.md](presentation/avatar_assets.md) | reviewed |
| [avatar_mode.ml](presentation/avatar_mode.ml) | component | [avatar_mode.md](presentation/avatar_mode.md) | reviewed |
| [content_cases.ml](presentation/content_cases.ml), [content_cases.mli](presentation/content_cases.mli) | component | [content_cases.md](presentation/content_cases.md) | reviewed |
| [main.ml](presentation/main.ml) | application-entry | [main.md](presentation/main.md) | reviewed |
| [rating_action.ml](presentation/rating_action.ml) | component | [rating_action.md](presentation/rating_action.md) | reviewed |
## progress

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](progress/main.ml) | application-entry | [main.md](progress/main.md) | reviewed |
## resource_audit

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [backend.ml](resource_audit/backend/backend.ml) | generated-registration | [backend.md](resource_audit/backend/backend.md) | reviewed |
| [registration.rs](resource_audit/backend/registration.rs) | generated-registration | [registration.md](resource_audit/backend/registration.md) | reviewed |
| [main.ml](resource_audit/main.ml) | diagnostic | [main.md](resource_audit/main.md) | reviewed |
| [gpuio_resource_audit.ml](resource_audit/ocaml/gpuio_resource_audit.ml), [gpuio_resource_audit.mli](resource_audit/ocaml/gpuio_resource_audit.mli) | diagnostic | [gpuio_resource_audit.md](resource_audit/ocaml/gpuio_resource_audit.md) | reviewed |
| [lib.rs](resource_audit/rust/src/lib.rs) | diagnostic | [lib.md](resource_audit/rust/src/lib.md) | reviewed |
| [metal.rs](resource_audit/rust/src/metal.rs) | diagnostic | [metal.md](resource_audit/rust/src/metal.md) | reviewed |
## runtime

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [bench.ml](runtime/bench.ml) | benchmark | [bench.md](runtime/bench.md) | reviewed |
| [bench_input.ml](runtime/bench_input.ml) | benchmark | [bench_input.md](runtime/bench_input.md) | reviewed |
| [main.ml](runtime/main.ml) | application-entry | [main.md](runtime/main.md) | reviewed |
## signal_studio

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [application.ml](signal_studio/application.ml), [application.mli](signal_studio/application.mli) | application-entry | [application.md](signal_studio/application.md) | reviewed |
| [application_identity.ml](signal_studio/application_identity.ml), [application_identity.mli](signal_studio/application_identity.mli) | component | [application_identity.md](signal_studio/application_identity.md) | reviewed |
| [backend.ml](signal_studio/backend/backend.ml) | generated-registration | [backend.md](signal_studio/backend/backend.md) | reviewed |
| [registration.rs](signal_studio/backend/registration.rs) | generated-registration | [registration.md](signal_studio/backend/registration.md) | reviewed |
| [checks.ml](signal_studio/checks.ml), [checks.mli](signal_studio/checks.mli) | test-support | [checks.md](signal_studio/checks.md) | reviewed |
| [component.ml](signal_studio/component.ml), [component.mli](signal_studio/component.mli) | component | [component.md](signal_studio/component.md) | reviewed |
| [documents.ml](signal_studio/documents.ml), [documents.mli](signal_studio/documents.mli) | component | [documents.md](signal_studio/documents.md) | reviewed |
| [document_file.ml](signal_studio/files/document_file.ml), [document_file.mli](signal_studio/files/document_file.mli) | component | [document_file.md](signal_studio/files/document_file.md) | reviewed |
| [document_file_test.ml](signal_studio/files/test/document_file_test.ml) | test-support | [document_file.md](signal_studio/files/document_file.md) | reviewed |
| [main.ml](signal_studio/main.ml) | application-entry | [main.md](signal_studio/main.md) | reviewed |
| [workspace_test.ml](signal_studio/model/test/workspace_test.ml) | test-support | [workspace.md](signal_studio/model/workspace.md) | reviewed |
| [workspace.ml](signal_studio/model/workspace.ml), [workspace.mli](signal_studio/model/workspace.mli) | support | [workspace.md](signal_studio/model/workspace.md) | reviewed |
| [run_alerts.ml](signal_studio/notifications/run_alerts.ml), [run_alerts.mli](signal_studio/notifications/run_alerts.mli) | component | [run_alerts.md](signal_studio/notifications/run_alerts.md) | reviewed |
| [run_alerts_test.ml](signal_studio/notifications/test/run_alerts_test.ml) | test-support | [run_alerts.md](signal_studio/notifications/run_alerts.md) | reviewed |
| [ui.ml](signal_studio/ui.ml), [ui.mli](signal_studio/ui.mli) | component | [ui.md](signal_studio/ui.md) | reviewed |
| [ui_thread.ml](signal_studio/ui_thread.ml), [ui_thread.mli](signal_studio/ui_thread.mli) | component | [ui_thread.md](signal_studio/ui_thread.md) | reviewed |
## table

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [event_actions.ml](table/event_actions.ml), [event_actions.mli](table/event_actions.mli) | component | [event_actions.md](table/event_actions.md) | reviewed |
| [main.ml](table/main.ml) | application-entry | [main.md](table/main.md) | reviewed |
## text_input

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](text_input/main.ml) | application-entry | [main.md](text_input/main.md) | reviewed |
## toasts

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](toasts/main.ml) | application-entry | [main.md](toasts/main.md) | reviewed |
## tooltips

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](tooltips/main.ml) | application-entry | [main.md](tooltips/main.md) | reviewed |
## tree

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [filesystem.ml](tree/filesystem.ml), [filesystem.mli](tree/filesystem.mli) | component | [filesystem.md](tree/filesystem.md) | reviewed |
| [filesystem_demo.ml](tree/filesystem_demo.ml), [filesystem_demo.mli](tree/filesystem_demo.mli) | component | [filesystem_demo.md](tree/filesystem_demo.md) | reviewed |
| [lifecycle_demo.ml](tree/lifecycle_demo.ml), [lifecycle_demo.mli](tree/lifecycle_demo.mli) | component | [lifecycle_demo.md](tree/lifecycle_demo.md) | reviewed |
| [main.ml](tree/main.ml) | application-entry | [main.md](tree/main.md) | reviewed |
| [outline_data.ml](tree/outline_data.ml), [outline_data.mli](tree/outline_data.mli) | component | [outline_data.md](tree/outline_data.md) | reviewed |
| [outline_demo.ml](tree/outline_demo.ml), [outline_demo.mli](tree/outline_demo.mli) | component | [outline_demo.md](tree/outline_demo.md) | reviewed |
| [outline_test.ml](tree/outline_test.ml) | test-support | [outline_test.md](tree/outline_test.md) | reviewed |
| [support.ml](tree/support.ml), [support.mli](tree/support.mli) | support | [support.md](tree/support.md) | reviewed |
## view_api

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [bonsai_component.ml](view_api/bonsai_component.ml) | component | [bonsai_component.md](view_api/bonsai_component.md) | reviewed |
| [components.ml](view_api/components.ml) | component | [components.md](view_api/components.md) | reviewed |
| [main.ml](view_api/main.ml) | application-entry | [main.md](view_api/main.md) | reviewed |
## virtual_list

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](virtual_list/main.ml) | application-entry | [main.md](virtual_list/main.md) | reviewed |
## window_lifecycle

| Source parts | Role | Walkthrough | Review |
| --- | --- | --- | --- |
| [main.ml](window_lifecycle/main.ml) | application-entry | [main.md](window_lifecycle/main.md) | reviewed |

417 source files in 260 groups; 260 reviewed, 0 pending.
