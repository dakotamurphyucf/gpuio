"""Scoped test-only NSPasteboard preservation, including all eagerly readable types.

Original payloads remain in memory and are never logged. Fail before changing the
clipboard if a promised representation cannot be materialized or exceeds bounds.
"""
import contextlib
import ctypes as C
import signal


class Pasteboard:
    def __init__(self):
        self.appkit = C.CDLL('/System/Library/Frameworks/AppKit.framework/AppKit')
        self.objc = C.CDLL('/usr/lib/libobjc.A.dylib')
        self.objc.objc_getClass.restype = C.c_void_p
        self.objc.objc_getClass.argtypes = [C.c_char_p]
        self.objc.sel_registerName.restype = C.c_void_p
        self.objc.sel_registerName.argtypes = [C.c_char_p]
        self.pool = self.call(self.call(self.cls('NSAutoreleasePool'), 'alloc'), 'init')
        self.board = self.call(self.cls('NSPasteboard'), 'generalPasteboard')

    def cls(self, name):
        result = self.objc.objc_getClass(name.encode())
        if not result:
            raise RuntimeError('Missing Objective-C class: ' + name)
        return result

    def call(self, receiver, selector, *arguments, result=C.c_void_p):
        signature = C.CFUNCTYPE(result, C.c_void_p, C.c_void_p, *(kind for kind, _ in arguments))
        function = signature(('objc_msgSend', self.objc))
        return function(receiver, self.objc.sel_registerName(selector.encode()), *(value for _, value in arguments))

    def string(self, value):
        return self.call(self.cls('NSString'), 'stringWithUTF8String:', (C.c_char_p, value.encode()))

    def text(self):
        value = self.call(self.board, 'stringForType:', (C.c_void_p, self.string('public.utf8-plain-text')))
        if not value:
            return None
        raw = self.call(value, 'UTF8String', result=C.c_char_p)
        return raw.decode() if raw is not None else None

    def snapshot(self):
        items = self.call(self.board, 'pasteboardItems')
        count = self.call(items, 'count', result=C.c_ulong) if items else 0
        if count > 32:
            raise RuntimeError('Clipboard has too many items to preserve for this test')
        saved, total = [], 0
        for index in range(count):
            item = self.call(items, 'objectAtIndex:', (C.c_ulong, index))
            types = self.call(item, 'types')
            length = self.call(types, 'count', result=C.c_ulong)
            if length > 64:
                raise RuntimeError('Clipboard has too many representations to preserve')
            representations = []
            for i in range(length):
                kind = self.call(types, 'objectAtIndex:', (C.c_ulong, i))
                name = self.call(kind, 'UTF8String', result=C.c_char_p).decode()
                data = self.call(item, 'dataForType:', (C.c_void_p, kind))
                if not data:
                    raise RuntimeError('Clipboard has an unavailable promised representation')
                size = self.call(data, 'length', result=C.c_ulong)
                total += size
                if total > 16 * 1024**2:
                    raise RuntimeError('Clipboard exceeds the 16 MiB test preservation budget')
                raw = self.call(data, 'bytes')
                representations.append((name, C.string_at(raw, size)))
            saved.append(sorted(representations))
        return saved

    def restore(self, snapshot):
        array = self.call(self.cls('NSMutableArray'), 'arrayWithCapacity:', (C.c_ulong, len(snapshot)))
        for representations in snapshot:
            item = self.call(self.call(self.cls('NSPasteboardItem'), 'alloc'), 'init')
            try:
                for name, raw in representations:
                    data = self.call(self.cls('NSData'), 'dataWithBytes:length:',
                                     (C.c_char_p, raw), (C.c_ulong, len(raw)))
                    if not self.call(item, 'setData:forType:', (C.c_void_p, data),
                                     (C.c_void_p, self.string(name)), result=C.c_bool):
                        raise RuntimeError('Cannot stage original clipboard representation')
                self.call(array, 'addObject:', (C.c_void_p, item), result=None)
            finally:
                self.call(item, 'release', result=None)
        self.call(self.board, 'clearContents', result=C.c_long)
        if snapshot and not self.call(self.board, 'writeObjects:', (C.c_void_p, array), result=C.c_bool):
            raise RuntimeError('Cannot restore original clipboard items')
        if self.snapshot() != snapshot:
            raise RuntimeError('Restored clipboard representations differ from original')

    def close(self):
        if self.pool:
            self.call(self.pool, 'drain', result=None)
            self.pool = None


@contextlib.contextmanager
def preserved_clipboard(factory=Pasteboard):
    board = factory()
    try:
        saved = board.snapshot()
        previous = signal.getsignal(signal.SIGTERM)
        def interrupted(signum, _frame):
            raise SystemExit(128 + signum)
        signal.signal(signal.SIGTERM, interrupted)
        try:
            yield board
        finally:
            try:
                board.restore(saved)
            finally:
                signal.signal(signal.SIGTERM, previous)
    finally:
        board.close()
