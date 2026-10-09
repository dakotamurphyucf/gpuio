"""Appearance matrix around the public searchable-list desktop walkthrough."""
import json


def exercise(mac, images=None):
    from gallery_selectable_list import exercise as exercise_search
    from mac_input_source import foreground_keys
    from test_gallery import TITLE, select_gallery_appearance, wait_absent

    report = {'complete': False, 'cases': []}
    original_post = mac.post_key
    try:
        foreground_keys(mac)
        mac.press(TITLE, 'Presentation')
        for current, following in (('Large', 'Compact'), ('Compact', 'Comfortable')):
            node = mac.find(TITLE, current, 'AXButton')
            if node:
                mac.release(node)
                mac.press(TITLE, current)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, following in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                case = {'theme': theme, 'scale': scale}
                report['cases'].append(case)
                destination = images / f'{theme}-{scale}' if images else None
                if destination:
                    destination.mkdir(parents=True, exist_ok=True)
                exercise_search(mac, destination)
                wait_absent(mac, 'Searchable catalog', 'AXList')
                wait_absent(mac, 'Search catalog', 'AXTextField')
                case['complete'] = True
                print('GALLERY_SEARCHABLE_CASE', theme, scale, 'PASS', flush=True)
                mac.press(TITLE, scale)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        report['complete'] = True
        print('GALLERY_SEARCHABLE_MATRIX_OK 6 cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'searchable-report.json').write_text(json.dumps(report, indent=2)+'\n')
