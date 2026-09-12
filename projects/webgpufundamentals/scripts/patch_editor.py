"""Make the upstream live editor find helpers under a nested Pages mount."""

OLD_PREFIX = "const prefix = `${dname}${dname.split('/').slice(4).map(() => '/..').join('')}`;"
MARKER = 'const yeokjaEditorRoot ='

def patch_editor(source):
    monaco = '`${window.location.origin}/monaco-editor/min/vs`'
    monaco_path = "new URL('../monaco-editor/min/vs', yeokjaEditorRoot + '/').href"
    if MARKER in source:
        return source.replace(monaco, monaco_path)
    if source.count(OLD_PREFIX) != 1:
        raise ValueError('upstream editor prefix changed; review its URL handling')
    declaration = "const yeokjaEditorRoot = new URL('..', document.currentScript.src).href.replace(/\\/$/, '');"
    source = source.replace("'use strict';", "'use strict';\n" + declaration, 1)
    return source.replace(OLD_PREFIX, 'const prefix = yeokjaEditorRoot;').replace(monaco, monaco_path)
