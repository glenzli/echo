#!/usr/bin/env python3
"""Run source QML pointer contracts in a disposable module; no app rebuild needed."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--input", default="apps/desktop/tests/qml", help="Focused QML test file or directory")
parser.add_argument("--native", action="store_true", help="Use the platform GPU renderer for icon color checks")
args = parser.parse_args()

root = Path(__file__).resolve().parents[1]
runner = shutil.which("qmltestrunner") or "/opt/homebrew/opt/qtdeclarative/bin/qmltestrunner"
with tempfile.TemporaryDirectory(prefix="echo-qml-contract-") as temporary:
    module = Path(temporary) / "EchoDesktop"
    module.mkdir()
    entries = ["module EchoDesktop", "singleton Theme 0.1 Theme.qml", "singleton SoundSemantics 0.1 SoundSemantics.qml"]
    for name in ["IndependentEditorHome.qml", "EchoWindowChrome.qml", "EchoWorkspaceTab.qml", "MainTitleBar.qml", "IndependentTitleBar.qml", "EchoButton.qml", "EchoIconButton.qml", "Theme.qml", "SoundSemantics.qml", "WaveformView.qml", "SoundAssemblyClip.qml", "SoundGainEnvelope.qml", "SoundAutomation.js", "SoundAssemblyEditing.js", "SoundAssemblySelection.js", "EchoIcon.qml", "EchoComboBox.qml", "SpectralEditing.js", "NoiseProfileEditing.js", "NoiseReductionPanel.qml", "EchoSegmentedControl.qml", "SpectrogramView.qml", "SpectralRepairSurface.qml", "SoundAdjustmentDraft.qml", "SoundEditorTimeline.qml", "TimeRangeDialog.qml", "Timecode.js", "SourceEditTimeline.qml", "SourceRegionInspector.qml", "EffectMaskEditor.qml", "EchoValueSpinBox.qml", "SoundAssemblyInspector.qml", "SoundDuckingPanel.qml", "EchoTimeSpinBox.qml", "EchoParameterSlider.qml", "EchoSwitch.qml", "EchoStereoMeter.qml", "SoundAssemblyTrack.qml", "EchoTextField.qml", "SoundAssemblyMarkers.qml", "SoundAssemblyMarkerLane.qml", "SoundTranscriptPanel.qml", "TranscriptExportMenu.qml", "SoundTranscriptEditing.js", "SourceEditRanges.js", "EchoCheckBox.qml", "SoundClickRepairPanel.qml", "ClickRepairEditing.js", "SourceDisclosure.js", "SourceDisclosureBadge.qml", "SourceDisclosureDialog.qml", "GeneratedNarrationDialog.qml", "SoundSourceBrowser.qml", "SoundSourceSearch.js", "SoundPlaybackSource.qml", "UnsavedEditDialog.qml", "AssemblyHistoryDialog.qml", "AssemblyVersionComparison.js", "MemoryInfoDialog.qml", "MemoryInfoSection.qml", "InspectorSection.qml", "AudioExportSettings.qml", "AssemblyExportDialog.qml", "AudioStreamImportDialog.qml"]:
        source = root / "apps/desktop/qml" / name
        contents = source.read_text()
        icon_prefix = "qrc:/EchoDesktop/icons/"
        if icon_prefix in contents:
            # Source-only tests have no compiled QRC; use the same icon files.
            (module / name).write_text(contents.replace(icon_prefix, (root / "apps/desktop/icons").as_uri() + "/"))
        else:
            (module / name).symlink_to(source)
        if name.endswith(".qml") and name not in ("Theme.qml", "SoundSemantics.qml"):
            entries.append(f"{Path(name).stem} 0.1 {name}")
    (module / "qmldir").write_text("\n".join(entries) + "\n")
    environment = dict(os.environ, QT_QUICK_CONTROLS_STYLE="Basic", QT_QPA_PLATFORM="offscreen", QSG_RHI_BACKEND="software", QT_QUICK_BACKEND="software")
    if args.native:
        environment.pop("QT_QUICK_BACKEND", None)
        environment.pop("QSG_RHI_BACKEND", None)
        environment.pop("QT_QPA_PLATFORM", None)
    raise SystemExit(subprocess.call([runner, "-input", str(root / args.input), "-import", temporary], env=environment))
