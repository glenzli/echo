import QtQuick
import QtQuick.Controls
import QtTest
import EchoDesktop

TestCase {
    id: test
    name: "EditorRecovery"
    width: 640; height: 500; visible: true; when: windowShown
    QtObject {
        id: controller
        property var recoverableSessions: []
        property string resumed: ""
        property int refreshes: 0
        property bool fail: false
        function refreshRecoverableSessions() { refreshes++; }
        function resumeSession(path) { resumed = path; return !fail; }
    }
    EditorRecoveryPicker { id: picker; anchors.centerIn: parent; controller: controller }
    function init() { picker.dialog.close(); controller.resumed = ""; controller.fail = false; controller.refreshes = 0; controller.recoverableSessions = []; }
    function entries(count) {
        const rows = [];
        for (let i = 0; i < count; ++i) rows.push({path: "/fixture/" + i, title: "雨声与旅行 " + i, sourceCount: 2, modifiedMillis: 1700000000000});
        return rows;
    }
    function test_large_history_is_bounded_and_last_project_is_reachable() {
        controller.recoverableSessions = entries(1000);
        tryCompare(picker, "visible", true);
        verify(picker.height < 60);
        mouseClick(findChild(picker, "editorRecoveryButton"));
        tryCompare(picker.dialog, "opened", true);
        compare(controller.refreshes, 1);
        verify(picker.dialog.height <= test.height - 32);
        verify(picker.dialog.width <= test.width - 32);
        const list = findChild(picker.dialog, "editorRecoveryList");
        tryVerify(function() { return list.height > 66 && list.height < picker.dialog.height; });
        verify(list.contentItem.children.length < 30);
        list.positionViewAtIndex(999, ListView.End);
        tryVerify(function() { return !!findChild(list, "editorRecoveryRow-999"); });
        mouseClick(findChild(list, "editorRecoveryRow-999"));
        compare(controller.resumed, "/fixture/999");
        tryCompare(picker.dialog, "visible", false);
    }
    function test_empty_history_is_hidden_and_failed_open_keeps_picker() {
        compare(picker.visible, false);
        controller.recoverableSessions = entries(1); controller.fail = true;
        mouseClick(findChild(picker, "editorRecoveryButton"));
        tryCompare(picker.dialog, "opened", true);
        const list = findChild(picker.dialog, "editorRecoveryList");
        tryCompare(list, "count", 1);
        tryVerify(function() { return list.height >= 66 && !!list.itemAtIndex(0); });
        mouseClick(list.itemAtIndex(0));
        verify(picker.dialog.visible); compare(controller.resumed, "/fixture/0");
        controller.recoverableSessions = [];
        compare(picker.visible, false);
    }
}
