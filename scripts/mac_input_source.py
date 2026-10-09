"""Scoped macOS Japanese input-source selection for foreground acceptance tests.

The recovery record is written before mutation. Restore also runs when the test
raises; SIGKILL/power loss requires the harness's explicit --restore command.
"""
import ctypes as C
from contextlib import contextmanager
import json

PARENT = 'com.apple.inputmethod.Kotoeri.RomajiTyping'
MODE = PARENT + '.Japanese'

class Sources:
    def __init__(self, mac):
        self.mac = mac
        self.carbon = C.CDLL('/System/Library/Frameworks/Carbon.framework/Carbon')
        def bind(name, result, *args):
            fn = getattr(self.carbon, name)
            fn.restype, fn.argtypes = result, args
            return fn
        ptr = C.c_void_p
        self.list_ = bind('TISCreateInputSourceList', ptr, ptr, C.c_bool)
        self.get = bind('TISGetInputSourceProperty', ptr, ptr, ptr)
        self.current = bind('TISCopyCurrentKeyboardInputSource', ptr)
        self.select = bind('TISSelectInputSource', C.c_int32, ptr)
        self.enable = bind('TISEnableInputSource', C.c_int32, ptr)
        self.disable = bind('TISDisableInputSource', C.c_int32, ptr)
        self.boolean = mac.cf.CFBooleanGetValue
        self.boolean.restype, self.boolean.argtypes = C.c_bool, [ptr]
        self.sources = self.list_(None, True)
        if not self.sources:
            raise RuntimeError('Cannot enumerate macOS input sources')
        self.by_id = {self.prop(mac.item(self.sources, i), 'InputSourceID'): mac.item(self.sources, i)
                      for i in range(mac.count(self.sources))}
    def prop(self, source, name):
        key = C.c_void_p.in_dll(self.carbon, 'kTISProperty' + name).value
        value = self.get(source, key)
        if not value:
            return None
        if name.startswith('InputSourceIs'):
            return bool(self.boolean(value))
        buffer = C.create_string_buffer(4096)
        if not self.mac.get_string(value, buffer, len(buffer), 0x08000100):
            raise RuntimeError('Cannot decode input source property ' + name)
        return buffer.value.decode()
    def selected(self):
        source = self.current()
        if not source:
            raise RuntimeError('No current macOS keyboard input source')
        try:
            return self.prop(source, 'InputSourceID')
        finally:
            self.mac.release(source)
    def enabled(self):
        return {id_: self.prop(source, 'InputSourceIsEnabled') for id_, source in self.by_id.items()}
    def checked(self, fn, id_):
        result = fn(self.by_id[id_])
        if result:
            raise RuntimeError(f'{fn.__name__}({id_}) failed: {result}')
    def restore(self, original, output):
        errors = []
        def attempt(fn, id_):
            try:
                self.checked(fn, id_)
            except Exception as error:
                errors.append(str(error))
        attempt(self.select, original['selected'])
        # Only this method's source family may be modified by the harness.
        # Restore children while their parent is enabled, then restore parent.
        for id_ in sorted(original['enabled'], key=len, reverse=True):
            if id_ != PARENT and not id_.startswith(PARENT + '.'):
                continue
            if self.prop(self.by_id[id_], 'InputSourceIsEnabled') != original['enabled'][id_]:
                attempt(self.enable if original['enabled'][id_] else self.disable, id_)
        # Re-enumerate rather than accepting a cached source-object observation.
        fresh = Sources(self.mac)
        try:
            after = {'selected': fresh.selected(), 'enabled': fresh.enabled()}
        finally:
            fresh.close()
        (output / 'restored.json').write_text(json.dumps(after, indent=2) + '\n')
        if after != original:
            errors.append('Input-source state differs from original.json')
        if errors:
            raise RuntimeError('Input-source restoration failed: ' + '; '.join(errors))
        print('IME_SOURCE_RESTORED', original['selected'], flush=True)
    def close(self):
        self.mac.release(self.sources)


@contextmanager
def japanese_source(sources, output):
    original = {'selected': sources.selected(), 'enabled': sources.enabled()}
    if PARENT not in original['enabled'] or MODE not in original['enabled']:
        raise RuntimeError('Installed Japanese Romaji/Hiragana input source is required')
    (output / 'original.json').write_text(json.dumps(original, indent=2) + '\n')
    try:
        sources.checked(sources.enable, PARENT)
        sources.checked(sources.enable, MODE)
        sources.checked(sources.select, MODE)
        if sources.selected() != MODE:
            raise RuntimeError('Japanese input-source selection did not take effect')
        yield
    finally:
        sources.restore(original, output)


def foreground_keys(mac):
    """Use the OS event route; refuse each event if the child lost foreground.

    The check cannot make focus and posting atomic. Run on an otherwise idle
    desktop. Direct-to-PID events did not activate the real IME in local testing.
    """
    post = mac.cg.CGEventPost
    post.restype, post.argtypes = None, [C.c_uint32, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    def checked_post(pid, event):
        if pid != mac.pid or mac.child.poll() is not None:
            raise RuntimeError('Owned test app is unavailable')
        front = mac.attr(mac.app, 'AXFrontmost')
        try:
            if not front or not boolean(front):
                raise RuntimeError('Owned test app lost foreground; refusing global key')
        finally:
            if front:
                mac.release(front)
        post(0, event)
    mac.post_key = checked_post
