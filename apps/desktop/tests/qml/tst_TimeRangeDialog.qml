import QtQuick
import QtTest
import EchoDesktop
import "../../qml/Timecode.js" as Timecode
TestCase {
    id: test; name: "TimeRangeDialog"
    width: 800; height: 700; visible: true; when: windowShown
    property var result: null
    TimeRangeDialog { id: dialog; minimumMillis: 1000; maximumMillis: 7200000; onRequested: (start,end) => test.result=[start,end] }
    function init() { dialog.rangeMode=true;dialog.contextKey="one";result=null; }
    function cleanup() { dialog.close(); }
    function test_formats_and_invalid_input() {
        compare(Timecode.parse(" 1:02:03.456 "),3723456);
        compare(Timecode.parse("90:00"),5400000);compare(Timecode.parse("12,25"),12250);
        compare(Timecode.parse("1：02.003"),62003);compare(Timecode.format(7200001),"2:00:00.001");
        for (const value of ["", "-1", "1e3", "NaN", "Infinity", "1:60", "1:60:00", "1.0001", "1:02:03:04", "999999999999"]) compare(Timecode.parse(value),null,value);
    }
    function test_range_keeps_millisecond_precision_and_does_not_clamp_invalid_text() {
        dialog.present(1000,10000);
        const start=findChild(dialog,"exactTimeStart"),end=findChild(dialog,"exactTimeEnd");
        start.text="1:23.456";end.text="1:25.789";
        mouseClick(findChild(dialog,"applyExactTime"));compare(result,[83456,85789]);verify(!dialog.visible);
        dialog.present(1000,10000);end.text="0:00.999";verify(!dialog.validRange);
        dialog.apply();verify(dialog.visible);compare(result,[83456,85789]);
        end.text="2:00:00.001";verify(!dialog.validRange);
    }
    function test_go_to_and_context_invalidation() {
        dialog.rangeMode=false;dialog.present(7000,7000);
        findChild(dialog,"exactTimeStart").text="1:30:00.001";
        keyClick(Qt.Key_Return);compare(result,[5400001,5400001]);
        dialog.present(1000,1000);dialog.contextKey="another-project";verify(!dialog.visible);
    }
}
