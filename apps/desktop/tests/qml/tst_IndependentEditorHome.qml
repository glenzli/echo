import QtQuick
import QtQuick.Controls
import QtTest
import EchoDesktop

TestCase {
    id: test
    name: "IndependentEditorHome"
    width: 960; height: 720; visible: true; when: windowShown
    QtObject {
        id: controller
        property var recentProjects: []
        property string opened: ""
        property bool recovered: false
        function openRecentProject(path, recovery) { opened = path; recovered = recovery; return true; }
    }
    IndependentEditorHome { id: home; anchors.fill: parent; controller: controller }
    SignalSpy { id: audio; target: home; signalName: "openAudioRequested" }
    SignalSpy { id: project; target: home; signalName: "openProjectRequested" }
    SignalSpy { id: narration; target: home; signalName: "narrationRequested" }
    function init() { controller.recentProjects = []; controller.opened = ""; controller.recovered = false; home.narrationAvailable = true; audio.clear(); project.clear(); narration.clear(); }
    function entries(count) {
        const result = [];
        for (let i = 0; i < count; ++i) result.push({path: "/fixture/" + i, title: "雨声与旅行 " + i, kind: i % 2 ? "recovery" : "project", sourceCount: 2, folder: "/旅行", modifiedMillis: 1700000000000});
        return result;
    }
    function test_actions_share_one_row_and_dispatch_the_right_flow() {
        const a = findChild(home, "editorStart-audio"), p = findChild(home, "editorStart-project"), n = findChild(home, "editorStart-narration");
        tryVerify(function() { return a.width > 100 && a.height === 82; });
        compare(a.y, p.y); compare(p.y, n.y);
        mouseClick(a); compare(audio.count, 1);
        mouseClick(p); compare(project.count, 1);
        home.narrationAvailable = false; mouseClick(n); compare(narration.count, 0);
        home.narrationAvailable = true; mouseClick(n); compare(narration.count, 1);
    }
    function test_large_recent_list_stays_bounded_and_last_item_is_reachable() {
        controller.recentProjects = entries(1000);
        const list = findChild(home, "editorRecentProjects");
        tryVerify(function() { return list.height > 100 && list.height < 400; });
        compare(list.count, 1000); verify(list.contentItem.children.length < 30);
        list.positionViewAtIndex(999, ListView.End);
        tryVerify(function() { return !!list.itemAtIndex(999); });
        mouseClick(list.itemAtIndex(999)); compare(controller.opened, "/fixture/999"); verify(controller.recovered);
    }
    function test_saved_project_opens_directly_without_recovery_popup() {
        controller.recentProjects = entries(2);
        const list = findChild(home, "editorRecentProjects");
        list.positionViewAtBeginning();
        tryVerify(function() { return !!list.itemAtIndex(0); });
        mouseClick(list.itemAtIndex(0)); compare(controller.opened, "/fixture/0"); verify(!controller.recovered);
    }
}
