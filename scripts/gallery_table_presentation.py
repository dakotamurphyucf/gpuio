"""Real-window scoped table paint, identity and input checks (not VoiceOver)."""
import ctypes as C
from pathlib import Path
import tempfile
import time

from test_canvas import screenshot
from window_pixels import read_png


def exercise(mac, images):
    from test_gallery import (TITLE, GalleryMouse, absent, element_rect,
                              reveal_gallery_control)
    from mac_input_source import foreground_keys

    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    mouse = GalleryMouse(mac)
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-table-presentation-')
    directory = images or Path(temporary.name)
    retained = []
    cases = []

    def rect(label, role='AXCell'):
        node = mac.wait_find(TITLE, label, role, search_files=True)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def leave_table():
        for _ in range(4):
            bounds = rect('Header and row styling', 'AXButton')
            x, y, w, h = bounds
            point = (x+w/2, y+h/2)
            mouse.check_owner(point)
            mouse.send(5, point)
            time.sleep(.2)
            settled = rect('Header and row styling', 'AXButton')
            print('TABLE_POINTER_OUTSIDE',point,'button',bounds,'after',settled,flush=True)
            if bounds == settled:
                return
        raise AssertionError('Table controls did not settle after layout change')

    def capture(tag):
        path = directory / f'table-presentation-{tag}.png'
        screenshot(mac, path, title=TITLE)
        window = mac.window(TITLE)
        try:
            wx, wy, ww, wh = element_rect(mac, window)
        finally:
            mac.release(window)
        pixels = read_png(mac, path)
        bx, by, bw, bh = rect('Inspect', 'AXButton')
        row_top = rect('0000')[1]
        assert by+bh <= row_top+.5, ('Header button escapes into body',tag,(bx,by,bw,bh),row_top)
        sx, sy = pixels.width/ww, pixels.height/wh
        foreground = (234,240,247) if tag.startswith('Dark-') else (27,41,57)
        ink = []
        for iy in range(2, round(bh*sy)-2):
            for ix in range(2, round(bw*sx)-2):
                color = pixels.rgb((bx-wx)*sx+ix, (by-wy)*sy+iy)
                if max(abs(a-b) for a,b in zip(color,foreground)) <= 30:
                    ink.append(iy/sy)
        assert ink, ('Header label did not paint',tag,(bx,by,bw,bh))
        print('TABLE_HEADER_LABEL',tag,'bounds',(bx,by,bw,bh),
              'ink_y',(min(ink),max(ink)),flush=True)
        assert min(ink) >= 2 and max(ink) <= bh-3, (
            'Header label reaches its clipped edge',tag,bh,min(ink),max(ink))
        bounds = [rect(f'{i:04d}') for i in range(4)]
        # Far right of the first column, away from text and selection borders.
        print('TABLE_CAPTURE_GEOMETRY',tag,'window',(wx,wy,ww,wh),'pixels',(pixels.width,pixels.height),'cells',bounds,flush=True)
        colors = [pixels.rgb((x+w-12-wx)*pixels.width/ww,
                             (y+h/2-wy)*pixels.height/wh)
                  for x, y, w, h in bounds]
        return bounds, colors

    def same_geometry(before, after):
        assert before == after, ('Style change moved table cells', before, after)
        current = mac.wait_find(TITLE, 'Inspect', 'AXButton', search_files=True)
        try:
            assert equal(retained[0], current), 'Rich header AX identity changed during paint update'
        finally:
            mac.release(current)

    try:
        mac.press(TITLE, 'Lists, trees & tables')
        mac.press(TITLE, 'Result table')
        reveal_gallery_control(mac, 'Preview results', 'AXTable')
        mac.press(TITLE, 'Rich table headers')
        reveal_gallery_control(mac, 'Preview results', 'AXTable')
        retained.append(mac.wait_find(TITLE, 'Inspect', 'AXButton', search_files=True))
        for theme in ('Dark', 'Light'):
            current = mac.find(TITLE, theme, 'AXButton')
            if current:
                mac.release(current)
            else:
                mac.press(TITLE, 'Light' if theme == 'Dark' else 'Dark')
                mac.release(mac.wait_find(TITLE, theme, 'AXButton'))
            for scale in ('Compact', 'Comfortable', 'Large'):
                for _ in range(3):
                    current = next((name for name in ('Compact','Comfortable','Large')
                                    if (node := mac.find(TITLE,name,'AXButton'))), None)
                    assert current is not None, 'Missing gallery scale control'
                    mac.release(node)
                    if current == scale:
                        break
                    mac.press(TITLE,current)
                    time.sleep(.12)
                else:
                    raise AssertionError('Scale did not settle')
                reveal_gallery_control(mac, 'Preview results', 'AXTable')
                leave_table()
                baseline, plain = capture(f'{theme}-{scale}-plain')
                current_header = mac.wait_find(TITLE,'Inspect','AXButton',search_files=True)
                print('TABLE_HEADER_AFTER_SCALE',theme,scale,'same_ax_identity',bool(equal(retained[0],current_header)),flush=True)
                mac.release(retained[0])
                retained[0] = current_header
                mac.press(TITLE, 'Header and row styling')
                leave_table()
                expected_base = (16,21,29) if theme == 'Dark' else (241,244,247)
                assert max(abs(a-b) for a,b in zip(plain[1],expected_base)) <= 3, ('Wrong rendered theme',theme,plain)
                styled, painted = capture(f'{theme}-{scale}-styled')
                same_geometry(baseline, styled)
                # Row zero has the public data-dependent accent tint; row one does not.
                accent = (137,221,201) if theme == 'Dark' else (9,110,91)
                expected_tint = tuple(round(b*.92+a*.08) for b,a in zip(plain[0],accent))
                assert max(abs(a-b) for a,b in zip(expected_tint,painted[0])) <= 3, (
                    'Data-dependent row tint did not paint', theme, plain, painted)
                assert max(abs(a-b) for a,b in zip(plain[1], painted[1])) <= 3, (
                    'Unstyled sibling changed', theme, plain, painted)
                x,y,w,h = styled[1]
                point = (x+w-12,y+h/2)
                mouse.check_owner(point)
                mouse.send(5, point)
                time.sleep(.12)
                hovered, hover_colors = capture(f'{theme}-{scale}-hover')
                same_geometry(styled, hovered)
                expected_hover = tuple(round(b*.84+a*.16) for b,a in zip(plain[1],accent))
                assert max(abs(a-b) for a,b in zip(expected_hover,hover_colors[1])) <= 3, (
                    'Row hover did not paint',theme,painted,hover_colors)
                leave_table()
                mac.press(TITLE, 'Header and row styling')
                leave_table()
                restored, reset = capture(f'{theme}-{scale}-reset')
                same_geometry(baseline, restored)
                assert all(max(abs(a-b) for a,b in zip(old,new)) <= 3
                           for old,new in zip(plain,reset)), ('Clearing styles did not restore paint',theme,plain,reset)
                cases.append({'theme':theme,'scale':scale,'baseline':plain,'styled':painted,'hover':hover_colors,'reset':reset})
                print('TABLE_PRESENTATION_PAINT', cases[-1], flush=True)
        mac.press(TITLE, 'Header and row styling')
        leave_table()
        x,y,w,h = rect('0001')
        point=(x+w/2,y+h/2)
        mouse.check_owner(point)
        mouse.send(5,point)
        try:
            mouse.send(1,point)
        finally:
            mouse.send(2,point)
        mac.wait_text(TITLE,'Table selection: Cell 1 / entry')
        original_post = mac.post_key
        try:
            foreground_keys(mac)
            mac.key(125)  # Down through the real foreground keyboard route.
        finally:
            mac.post_key = original_post
        mac.wait_text(TITLE,'Table selection: Cell 2 / entry')
        bx, by, bw, bh = rect('Inspect', 'AXButton')
        point = (bx+bw/2, by+bh/2)
        mouse.check_owner(point)
        mouse.send(5, point)
        try:
            mouse.send(1, point)
        finally:
            mouse.send(2, point)
        mac.wait_text(TITLE,'Header action handled independently of table sorting')
        # Change the notice through a distinct row before verifying keyboard action.
        x,y,w,h = rect('0003')
        point = (x+w/2,y+h/2)
        mouse.check_owner(point)
        mouse.send(5,point)
        try:
            mouse.send(1,point)
        finally:
            mouse.send(2,point)
        mac.wait_text(TITLE,'Table selection: Cell 3 / entry')
        mac.wait_text(TITLE,'Cell 3 / entry selected')
        mac.set(retained[0], 'AXFocused', mac.true)
        boolean = mac.cf.CFBooleanGetValue
        boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
        deadline = time.monotonic()+5
        while True:
            focused = mac.attr(retained[0], 'AXFocused')
            try:
                if focused and boolean(focused):
                    break
            finally:
                if focused:
                    mac.release(focused)
            assert time.monotonic() < deadline, 'Header action did not take native focus'
            time.sleep(.025)
        original_post = mac.post_key
        try:
            foreground_keys(mac)
            mac.key(49)  # Space activates the focused native button.
        finally:
            mac.post_key = original_post
        mac.wait_text(TITLE,'Header action handled independently of table sorting')
        mac.press(TITLE,'Header and row styling')
        mac.wait_text(TITLE,'Table selection: Cell 3 / entry')
        mac.press(TITLE,'Presentation')
        mac.wait_text(TITLE,'A little context goes a long way')
        absent(mac,'Preview results','AXTable')
        mac.press(TITLE,'Lists, trees & tables')
        mac.press(TITLE,'Result table')
        reveal_gallery_control(mac,'Preview results','AXTable')
        mac.wait_text(TITLE,'Table selection: Nothing')
        mac.press(TITLE,'Presentation')
        mac.wait_text(TITLE,'A little context goes a long way')
        absent(mac,'Preview results','AXTable')
        print('GALLERY_TABLE_PRESENTATION_OK: two themes by three scales, scoped row tint/hover/reset pixels, '
              'stable geometry and rich-header identity, real pointer/Down selection, independent '
              'header pointer/Space action and unclipped label, selection retention and page remount',flush=True)
    finally:
        for node in retained:
            mac.release(node)
        temporary.cleanup()
