"""Actual gallery header bounds across public pages, scales and window sizes."""
import ctypes as C
import json
import tempfile
import time
from pathlib import Path

from test_canvas import screenshot
from test_gallery import TITLE, GalleryMouse, element_rect

PAGES = [
    'Presentation', 'Settings', 'Styling details', 'Selection & actions',
    'Text editing', 'Numbers & codes', 'Dates & colors', 'Overlays & help',
    'Navigation & layout', 'Commands & feedback', 'Carousels & journeys',
    'Lists, trees & tables', 'Markdown & code', 'Find & highlight',
    'Canvas & drawing', 'Images & icons', 'Charts & data', 'Motion & rhythm',
    'Native extensions', 'Input & transfers', 'Input observations',
    'Responsive layouts', 'Desktop services', 'Runtime & windows',
]
SCALES = ('Compact', 'Comfortable', 'Large')


def exercise(mac, images):
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-header-')
    output = images or Path(temporary.name)
    cases = []
    def bounds(label, role='AXButton'):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac,node)
        finally:
            mac.release(node)
    def current(options):
        for name in options:
            node = mac.find(TITLE,name,'AXButton')
            if node:
                mac.release(node)
                return name
        raise AssertionError(('Missing shell control', options))
    def scale(wanted):
        for _ in range(3):
            value = current(SCALES)
            if value == wanted:
                return
            mac.press(TITLE,value)
            mac.release(mac.wait_find(TITLE,SCALES[(SCALES.index(value)+1)%3],'AXButton'))
        assert current(SCALES) == wanted
    def resize(width):
        window = mac.window(TITLE)
        create = mac.ax.AXValueCreate
        create.restype,create.argtypes=C.c_void_p,[C.c_int,C.c_void_p]
        size=(C.c_double*2)(width,820)
        value=create(2,C.byref(size))
        try:
            mac.set(window,'AXSize',value)
            deadline=time.monotonic()+5
            while abs(element_rect(mac,window)[2]-width)>.5:
                assert time.monotonic()<deadline, 'Window resize did not settle'
                time.sleep(.03)
        finally:
            mac.release(value)
            mac.release(window)
        time.sleep(.15)
    def verify(page, theme, density, width, capture=False):
        mac.press(TITLE,page)
        heading = bounds(page,'AXStaticText')
        time.sleep(.12)
        window=mac.window(TITLE)
        try:
            viewport=element_rect(mac,window)
        finally:
            mac.release(window)
        labels=[theme,'Follow system',density,'New window']
        controls=[bounds(label) for label in labels]
        print('HEADER_BOUNDS',page,theme,density,width,viewport,controls,flush=True)
        if capture:
            screenshot(mac,output/f'header-{theme}-{density}-{width}.png',title=TITLE)
        for label,(x,y,w,h) in zip(labels,controls):
            assert w>10 and h>10, (label,'Empty header control')
            assert x>=viewport[0] and x+w<=viewport[0]+viewport[2]-.5, (
                label,'Header control escapes window',viewport,(x,y,w,h))
            assert y>=viewport[1] and y+h<=viewport[1]+viewport[3], (label,'Vertical overflow')
        for i,a in enumerate(controls):
            for b in controls[i+1:]+[heading]:
                overlap_x=min(a[0]+a[2],b[0]+b[2])-max(a[0],b[0])
                overlap_y=min(a[1]+a[3],b[1]+b[3])-max(a[1],b[1])
                assert overlap_x<=.5 or overlap_y<=.5, ('Overlapping header',page,a,b)
        cases.append(dict(page=page,theme=theme,scale=density,width=width,controls=controls))
    try:
        mac.wait_text(TITLE,'A little context goes a long way')
        mac.set(mac.app,'AXFrontmost',mac.true)
        scale('Large')
        # Retain a direct before-case for the observed clipped New window button.
        verify('Markdown & code',current(('Dark','Light')),'Large',1120,capture=True)
        for width in (900,1120):
            resize(width)
            for theme in ('Dark','Light'):
                value=current(('Dark','Light'))
                if value!=theme:
                    mac.press(TITLE,value)
                    mac.release(mac.wait_find(TITLE,theme,'AXButton'))
                for density in SCALES:
                    scale(density)
                    pages=PAGES if density=='Large' and theme=='Dark' else ['Markdown & code']
                    for page in pages:
                        verify(page,theme,density,width,capture=page=='Markdown & code')
        # The previously clipped control must accept a physical pointer click.
        x,y,w,h=bounds('New window')
        mouse=GalleryMouse(mac);point=(x+w/2,y+h/2)
        mouse.check_owner(point);mouse.send(5,point);mouse.send(1,point);mouse.send(2,point)
        from test_gallery import SECOND
        mac.wait_text(SECOND,'A little context goes a long way')
        mac.close(SECOND)
        (output/'header-report.json').write_text(json.dumps(cases,indent=2)+'\n')
        print('GALLERY_HEADER_LAYOUT_OK',len(cases),'cases and real New window activation',flush=True)
    finally:
        temporary.cleanup()
