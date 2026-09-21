import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test; name: "UnsavedEdit"; width: 800; height: 600; visible: true; when: windowShown
    property int continuations: 0
    QtObject {
        id: editor
        property bool dirty: true
        property bool failSave: false
        property int saves: 0
        property QtObject adjustment: QtObject { function revert() { editor.dirty=false; } }
        function save() { ++saves; if (!failSave) dirty=false; }
    }
    UnsavedEditDialog { id: dialog; editor: editor }
    function init() { dialog.close(); tryCompare(dialog,"visible",false); editor.dirty=true; editor.failSave=false; editor.saves=0; continuations=0; }
    function navigate() { dialog.request(()=>++test.continuations); tryCompare(dialog,"opened",true); waitForRendering(dialog.contentItem); }
    function test_cancel_preserves_draft_and_destination() {
        navigate(); mouseClick(findChild(dialog,"keepEditing")); tryCompare(dialog,"visible",false);
        verify(editor.dirty); compare(continuations,0); compare(editor.saves,0);
        editor.dirty=false; dialog.request(()=>++test.continuations); compare(continuations,1);
    }
    function test_failed_save_stays_until_successful_retry() {
        navigate(); editor.failSave=true; mouseClick(findChild(dialog,"saveEdits"));
        verify(dialog.visible); verify(dialog.saveFailed); compare(continuations,0); verify(editor.dirty);
        editor.failSave=false; mouseClick(findChild(dialog,"saveEdits"));
        tryCompare(dialog,"visible",false); compare(continuations,1); verify(!editor.dirty);
    }
    function test_discard_is_explicit_and_does_not_publish() {
        navigate(); mouseClick(findChild(dialog,"discardEdits"));
        tryCompare(dialog,"visible",false); compare(continuations,1); compare(editor.saves,0); verify(!editor.dirty);
    }
}
