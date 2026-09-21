import QtQuick
import QtQuick.Controls
import QtTest
import EchoDesktop
TestCase {
    id: test
    name: "MemoryInfo"
    width: 800; height: 800; visible: true; when: windowShown
    QtObject {
        id: catalog
        property var records: ({})
        property bool failSave: false
        property bool failLoad: false
        signal assetsChanged()
        function memoryInfo(id, assembly) {
            if (failLoad) return {error: "Could not load"};
            return records[id] || {revision: 0, info: {notes: "", place: "", timeDescription: "", moments: []}};
        }
        function setMemoryInfo(id, assembly, expected, info) {
            if (failSave || memoryInfo(id, assembly).revision !== expected) return "Changed in another window";
            const next = Object.assign({}, records);
            next[id] = {revision: expected + 1, info: JSON.parse(JSON.stringify(info))};
            records = next; assetsChanged(); return "";
        }
    }
    MemoryInfoDialog { id: dialog; catalogBackend: catalog }
    AudioExportSettings { id: settings; width: 500 }
    function init() { dialog.close(); catalog.records = {}; catalog.failSave = false; catalog.failLoad = false; settings.configure({format: "wav_pcm24"}); }
    function test_context_and_moments_save_and_reopen() {
        dialog.present("source", false, 100, 250); tryCompare(dialog, "opened", true);
        dialog.notes = "私人备注\n第二行"; dialog.place = "外婆家阳台"; dialog.timeDescription = "大约 2020 年夏天";
        dialog.addMoment(); compare(dialog.moments.length, 1);
        dialog.moments[0].note = "第一次叫爸爸";
        mouseClick(findChild(dialog, "saveMemoryInfo")); tryCompare(dialog, "visible", false);
        dialog.present("source", false); compare(dialog.notes, "私人备注\n第二行"); compare(dialog.place, "外婆家阳台");
        compare(dialog.moments[0].startMillis, 100); compare(dialog.moments[0].endMillis, 250);
        verify(dialog.width <= test.width && dialog.height <= test.height);
    }
    function test_conflict_and_cancel_keep_draft_without_overwriting() {
        dialog.present("source", false); dialog.notes = "unsaved"; catalog.failSave = true;
        dialog.save(); verify(dialog.visible); compare(dialog.notes, "unsaved"); verify(!!dialog.errorText);
        dialog.close(); catalog.failSave = false; dialog.present("source", false); compare(dialog.notes, "");
        catalog.failLoad = true; dialog.present("source", false); verify(!findChild(dialog, "saveMemoryInfo").enabled);
    }
    function test_assembly_context_is_separate_and_export_is_opt_in() {
        dialog.present("mix", true, 100, 200); dialog.addMoment(); compare(dialog.moments.length, 0);
        dialog.notes = "Whole trip"; dialog.save(); dialog.present("source", false); compare(dialog.notes, "");
        compare(settings.options.includeMemoryInfo, false);
        settings.configure({format: "mp3", includeMemoryInfo: true}); compare(settings.options.includeMemoryInfo, true);
        settings.configure({format: "aac_m4a"}); compare(settings.options.includeMemoryInfo, false);
    }
    function test_empty_moment_retains_editor_until_corrected() {
        dialog.present("source", false, 10, -1); dialog.addMoment(); dialog.save(); verify(dialog.visible); verify(!!dialog.errorText);
        dialog.moments[0].note = "Moment"; dialog.save(); verify(!dialog.visible);
    }
}
