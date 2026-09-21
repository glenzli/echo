import QtQuick
import QtTest
import EchoDesktop

TestCase {
    id: test
    name: "DraftLifecycle"
    property int dirtyPublications: 0
    SoundAdjustmentDraft {
        id: draft
        asset: null
        onDirtyChanged: if (dirty) ++test.dirtyPublications
    }
    function test_loading_sources_never_publishes_an_edit() {
        draft.asset = null;
        dirtyPublications = 0;
        draft.asset = {id: "one", path: "one.wav", durationMillis: 10000, adjustmentRevision: 1, gainCentibels: -200, trimStartMillis: 0, fadeInMillis: 0, fadeOutMillis: 0, lowCutHertz: 0};
        verify(!draft.dirty);
        compare(dirtyPublications, 0);
        draft.asset = {id: "two", path: "two.wav", durationMillis: 20000, adjustmentRevision: 4, gainCentibels: -600, trimStartMillis: 0, fadeInMillis: 0, fadeOutMillis: 0, lowCutHertz: 0};
        verify(!draft.dirty);
        compare(dirtyPublications, 0);
        draft.setGain(-900);
        verify(draft.dirty);
        compare(dirtyPublications, 1);
        draft.undo();
        verify(!draft.dirty);
        draft.redo();
        verify(draft.dirty);
        draft.markSaved();
        verify(!draft.dirty);
        dirtyPublications = 0;
        draft.resetFromAsset();
        verify(!draft.dirty);
        compare(dirtyPublications, 0);
    }
}
