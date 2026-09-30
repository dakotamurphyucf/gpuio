"""Read macOS test-window PNG pixels without an optional Python image package."""
import ctypes as C
import os


class Rect(C.Structure):
    _fields_ = [('x', C.c_double), ('y', C.c_double),
                ('width', C.c_double), ('height', C.c_double)]


class Pixels:
    def __init__(self, width, height, data):
        self.width, self.height, self.data = width, height, data

    def rgb(self, x, y):
        x, y = int(x), int(y)
        assert 0 <= x < self.width and 0 <= y < self.height, (x, y)
        offset = (y * self.width + x) * 4
        return tuple(self.data[offset:offset + 3])


def read_png(mac, path):
    """Decode to top-left-origin RGBA8 through system ImageIO/CoreGraphics."""
    image_io = C.CDLL('/System/Library/Frameworks/ImageIO.framework/ImageIO')
    ptr = C.c_void_p

    def bind(lib, name, result, *args):
        function = getattr(lib, name)
        function.restype, function.argtypes = result, args
        return function

    make_url = bind(mac.cf, 'CFURLCreateFromFileSystemRepresentation', ptr,
                    ptr, C.c_char_p, C.c_long, C.c_bool)
    source_from_url = bind(image_io, 'CGImageSourceCreateWithURL', ptr, ptr, ptr)
    image_at = bind(image_io, 'CGImageSourceCreateImageAtIndex', ptr, ptr, C.c_size_t, ptr)
    width_of = bind(mac.cg, 'CGImageGetWidth', C.c_size_t, ptr)
    height_of = bind(mac.cg, 'CGImageGetHeight', C.c_size_t, ptr)
    color_space = bind(mac.cg, 'CGColorSpaceCreateDeviceRGB', ptr)
    context = bind(mac.cg, 'CGBitmapContextCreate', ptr, ptr, C.c_size_t, C.c_size_t,
                   C.c_size_t, C.c_size_t, ptr, C.c_uint32)
    draw = bind(mac.cg, 'CGContextDrawImage', None, ptr, Rect, ptr)
    encoded_path = os.fsencode(path.resolve())
    resources = []

    def own(value):
        assert value, f'Cannot decode window PNG: {path}'
        resources.append(value)
        return value

    try:
        url = own(make_url(None, encoded_path, len(encoded_path), False))
        source = own(source_from_url(url, None))
        image = own(image_at(source, 0, None))
        width, height = width_of(image), height_of(image)
        assert 0 < width * height <= 16_000_000, (width, height)
        data = (C.c_ubyte * (width * height * 4))()
        space = own(color_space())
        # Big-endian 32-bit RGBA with premultiplied alpha. Window captures are opaque.
        bitmap = own(context(data, width, height, 8, width * 4, space, (4 << 12) | 1))
        draw(bitmap, Rect(0, 0, width, height), image)
        return Pixels(width, height, data)
    finally:
        for resource in reversed(resources):
            mac.release(resource)
