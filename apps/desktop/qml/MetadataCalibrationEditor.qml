//! In-place editor for user corrections over model-derived sound metadata.
//! Model evidence remains visible and immutable; this component only emits
//! the complete desired presentation values for an append-only calibration.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: editor

    property var asset: null
    property bool saving: false
    property string errorText: ""
    property var calibrationIntent: ({})
    property bool loadingFields: false

    signal saveRequested(var values)
    signal cancelRequested

    color: Theme.panelRaised

    function calibrated(field: string): bool {
        if (!asset || !asset.calibratedFields)
            return false;
        for (const value of asset.calibratedFields) {
            if (String(value) === field)
                return true;
        }
        return false;
    }

    function keywordsText(values: var): string {
        if (!values)
            return "";
        const result = [];
        for (const value of values)
            result.push(String(value));
        return result.join(" · ");
    }

    function parsedKeywords(): var {
        const result = [];
        const seen = {};
        for (const candidate of keywordInput.text.split(/[，,;；\n·]+/)) {
            const value = candidate.trim();
            const key = value.toLocaleLowerCase();
            if (value.length > 0 && !seen[key]) {
                seen[key] = true;
                result.push(value);
            }
        }
        return result;
    }

    function setFieldIntent(field: string, calibrated: bool): void {
        const next = {};
        for (const key in calibrationIntent)
            next[key] = Boolean(calibrationIntent[key]);
        next[field] = calibrated;
        calibrationIntent = next;
    }

    function resetField(field: string, control: var, modelValue: string): void {
        loadingFields = true;
        control.text = modelValue;
        loadingFields = false;
        setFieldIntent(field, false);
        errorText = "";
    }

    function keywordsEqual(left: var, right: var): bool {
        if (!left || !right || left.length !== right.length)
            return false;
        for (let index = 0; index < left.length; ++index) {
            if (String(left[index]) !== String(right[index]))
                return false;
        }
        return true;
    }

    function intendedFields(values: var): var {
        const result = [];
        const modelKeywords = asset && asset.modelKeywords ? asset.modelKeywords : [];
        const differs = {
            sound_caption: !asset || values.soundCaption !== String(asset.modelSoundCaption || ""),
            summary: !asset || values.summary !== String(asset.modelSummary || ""),
            event_type: !asset || values.eventType !== String(asset.modelEventType || ""),
            mood: !asset || values.mood !== String(asset.modelMood || ""),
            keywords: !keywordsEqual(values.keywords, modelKeywords),
            transcript_text: !asset || values.transcriptText !== String(asset.modelTextPreview || ""),
            language: !asset || values.language !== String(asset.modelLanguage || "")
        };
        for (const field of ["sound_caption", "summary", "event_type", "mood", "keywords", "transcript_text", "language"]) {
            if (Boolean(calibrationIntent[field]) || differs[field])
                result.push(field);
        }
        return result;
    }

    function loadAsset(): void {
        errorText = "";
        if (!asset)
            return;
        loadingFields = true;
        captionInput.text = String(asset.soundCaption || "");
        summaryInput.text = String(asset.summary || "");
        eventInput.text = String(asset.eventType || "");
        moodInput.text = String(asset.mood || "");
        keywordInput.text = keywordsText(asset.keywords);
        transcriptInput.text = String(asset.textPreview || "");
        languageInput.text = String(asset.language || "");
        loadingFields = false;
        const nextIntent = {};
        if (asset.calibratedFields) {
            for (const field of asset.calibratedFields)
                nextIntent[String(field)] = true;
        }
        calibrationIntent = nextIntent;
    }

    function restoreModelValues(): void {
        if (!asset)
            return;
        loadingFields = true;
        captionInput.text = String(asset.modelSoundCaption || "");
        summaryInput.text = String(asset.modelSummary || "");
        eventInput.text = String(asset.modelEventType || "");
        moodInput.text = String(asset.modelMood || "");
        keywordInput.text = keywordsText(asset.modelKeywords);
        transcriptInput.text = String(asset.modelTextPreview || "");
        languageInput.text = String(asset.modelLanguage || "");
        loadingFields = false;
        calibrationIntent = {};
        errorText = "";
    }

    function submit(): void {
        errorText = "";
        if (summaryInput.text.length > 1000) {
            errorText = qsTr("The summary must be 1,000 characters or fewer");
            return;
        }
        if (transcriptInput.text.length > 131072) {
            errorText = qsTr("The text is too long to save as one sound record");
            return;
        }
        if (parsedKeywords().length > 32) {
            errorText = qsTr("Use no more than 32 keywords");
            return;
        }
        const values = {
            soundCaption: captionInput.text,
            summary: summaryInput.text,
            eventType: eventInput.text,
            mood: moodInput.text,
            keywords: parsedKeywords(),
            transcriptText: transcriptInput.text,
            language: languageInput.text
        };
        values.calibratedFields = intendedFields(values);
        saving = true;
        saveRequested(values);
    }

    onAssetChanged: {
        if (visible)
            Qt.callLater(loadAsset);
    }
    onVisibleChanged: {
        if (visible)
            Qt.callLater(loadAsset);
    }

    component FieldHeader: RowLayout {
        required property string label
        required property bool isCalibrated
        required property string modelText
        signal resetRequested

        spacing: 6

        Text {
            Layout.fillWidth: true
            text: parent.label
            color: Theme.textPrimary
            font.pixelSize: Theme.fontBody
        }

        Rectangle {
            visible: parent.isCalibrated
            implicitWidth: calibratedLabel.implicitWidth + 10
            implicitHeight: 20
            radius: 6
            color: Theme.accentSurfaceQuiet

            Text {
                id: calibratedLabel
                anchors.centerIn: parent
                text: qsTr("Calibrated")
                color: Theme.accentSelectionText
                font.pixelSize: Theme.fontMeta
            }
        }

        EchoIconButton {
            visible: parent.isCalibrated
            source: "qrc:/EchoDesktop/icons/refresh.svg"
            toolTipText: qsTr("Use model suggestion")
            buttonSize: 24
            iconSize: 12
            onClicked: parent.resetRequested()
        }
    }

    component ModelHint: Text {
        required property bool calibrated
        required property string modelText
        Layout.fillWidth: true
        visible: calibrated
        text: modelText.length > 0 ? qsTr("Model suggestion: %1").arg(modelText) : qsTr("The model did not provide a value")
        color: Theme.textDisabled
        font.pixelSize: Theme.fontMeta
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
    }

    component StyledArea: TextArea {
        id: area
        Layout.fillWidth: true
        color: Theme.textPrimary
        placeholderTextColor: Theme.textDisabled
        selectionColor: Theme.accentSurface
        selectedTextColor: Theme.accentSelectionText
        selectByMouse: true
        wrapMode: TextEdit.Wrap
        padding: 10

        background: Rectangle {
            radius: Theme.controlRadius
            border.width: 1
            border.color: area.activeFocus ? Theme.focusRing : Theme.buttonBorder
            color: Theme.control
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 52
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            spacing: 8

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/chevron-down.svg"
                rotation: 90
                toolTipText: qsTr("Cancel")
                buttonSize: 28
                iconSize: 14
                onClicked: editor.cancelRequested()
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Calibrate information")
                    color: Theme.textPrimary
                    font.pixelSize: 15
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("The original model result remains available as evidence")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideRight
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Restore all model suggestions")
                buttonSize: 28
                iconSize: 14
                onClicked: editor.restoreModelValues()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ScrollView {
            id: formScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: formScroll.availableWidth
                spacing: 8

                Item {
                    Layout.preferredHeight: 5
                }

                FieldHeader {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    label: qsTr("Sound title")
                    isCalibrated: editor.calibrated("sound_caption")
                    modelText: editor.asset ? String(editor.asset.modelSoundCaption || "") : ""
                    onResetRequested: editor.resetField("sound_caption", captionInput, modelText)
                }
                EchoTextField {
                    id: captionInput
                    objectName: "metadataCaptionInput"
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    maximumLength: 120
                    placeholderText: qsTr("A short, recognizable sound title")
                    onTextEdited: editor.setFieldIntent("sound_caption", true)
                }
                ModelHint {
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    calibrated: editor.calibrated("sound_caption")
                    modelText: editor.asset ? String(editor.asset.modelSoundCaption || "") : ""
                }

                FieldHeader {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    label: qsTr("Summary")
                    isCalibrated: editor.calibrated("summary")
                    modelText: editor.asset ? String(editor.asset.modelSummary || "") : ""
                    onResetRequested: editor.resetField("summary", summaryInput, modelText)
                }
                StyledArea {
                    id: summaryInput
                    objectName: "metadataSummaryInput"
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    Layout.preferredHeight: 86
                    placeholderText: qsTr("Briefly describe what can be heard")
                    onTextChanged: {
                        if (!editor.loadingFields)
                            editor.setFieldIntent("summary", true);
                    }
                }
                ModelHint {
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    calibrated: editor.calibrated("summary")
                    modelText: editor.asset ? String(editor.asset.modelSummary || "") : ""
                }

                GridLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    columns: 2
                    columnSpacing: 10
                    rowSpacing: 8

                    ColumnLayout {
                        Layout.fillWidth: true
                        FieldHeader {
                            Layout.fillWidth: true
                            label: qsTr("Event")
                            isCalibrated: editor.calibrated("event_type")
                            modelText: editor.asset ? String(editor.asset.modelEventType || "") : ""
                            onResetRequested: editor.resetField("event_type", eventInput, modelText)
                        }
                        EchoTextField {
                            id: eventInput
                            objectName: "metadataEventInput"
                            Layout.fillWidth: true
                            maximumLength: 80
                            placeholderText: qsTr("Sound event")
                            onTextEdited: editor.setFieldIntent("event_type", true)
                        }
                        ModelHint {
                            calibrated: editor.calibrated("event_type")
                            modelText: editor.asset ? String(editor.asset.modelEventType || "") : ""
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        FieldHeader {
                            Layout.fillWidth: true
                            label: qsTr("Mood")
                            isCalibrated: editor.calibrated("mood")
                            modelText: editor.asset ? String(editor.asset.modelMood || "") : ""
                            onResetRequested: editor.resetField("mood", moodInput, modelText)
                        }
                        EchoTextField {
                            id: moodInput
                            objectName: "metadataMoodInput"
                            Layout.fillWidth: true
                            maximumLength: 80
                            placeholderText: qsTr("Mood or tone")
                            onTextEdited: editor.setFieldIntent("mood", true)
                        }
                        ModelHint {
                            calibrated: editor.calibrated("mood")
                            modelText: editor.asset ? String(editor.asset.modelMood || "") : ""
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.columnSpan: 2
                        FieldHeader {
                            Layout.fillWidth: true
                            label: qsTr("Language")
                            isCalibrated: editor.calibrated("language")
                            modelText: editor.asset ? String(editor.asset.modelLanguage || "") : ""
                            onResetRequested: editor.resetField("language", languageInput, modelText)
                        }
                        EchoTextField {
                            id: languageInput
                            objectName: "metadataLanguageInput"
                            Layout.fillWidth: true
                            maximumLength: 80
                            placeholderText: qsTr("Language, for example zh or en")
                            onTextEdited: editor.setFieldIntent("language", true)
                        }
                        ModelHint {
                            calibrated: editor.calibrated("language")
                            modelText: editor.asset ? String(editor.asset.modelLanguage || "") : ""
                        }
                    }
                }

                FieldHeader {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    label: qsTr("Keywords")
                    isCalibrated: editor.calibrated("keywords")
                    modelText: editor.asset ? editor.keywordsText(editor.asset.modelKeywords) : ""
                    onResetRequested: editor.resetField("keywords", keywordInput, modelText)
                }
                EchoTextField {
                    id: keywordInput
                    objectName: "metadataKeywordInput"
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    placeholderText: qsTr("Separate keywords with commas")
                    onTextEdited: editor.setFieldIntent("keywords", true)
                }
                ModelHint {
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    calibrated: editor.calibrated("keywords")
                    modelText: editor.asset ? editor.keywordsText(editor.asset.modelKeywords) : ""
                }

                FieldHeader {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    label: qsTr("Text")
                    isCalibrated: editor.calibrated("transcript_text")
                    modelText: editor.asset ? String(editor.asset.modelTextPreview || "") : ""
                    onResetRequested: editor.resetField("transcript_text", transcriptInput, modelText)
                }
                StyledArea {
                    id: transcriptInput
                    objectName: "metadataTranscriptInput"
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    Layout.preferredHeight: 180
                    placeholderText: qsTr("Correct or add the words heard in this sound")
                    onTextChanged: {
                        if (!editor.loadingFields)
                            editor.setFieldIntent("transcript_text", true);
                    }
                }
                ModelHint {
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    calibrated: editor.calibrated("transcript_text")
                    modelText: editor.asset ? String(editor.asset.modelTextPreview || "") : ""
                }

                Item {
                    Layout.preferredHeight: 8
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 9
            Layout.bottomMargin: 10
            spacing: 7

            Text {
                Layout.fillWidth: true
                visible: editor.errorText.length > 0
                text: editor.errorText
                color: Theme.warningText
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Item {
                    Layout.fillWidth: true
                }

                EchoButton {
                    text: qsTr("Cancel")
                    ghost: true
                    enabled: !editor.saving
                    onClicked: editor.cancelRequested()
                }

                EchoButton {
                    objectName: "metadataSaveButton"
                    text: editor.saving ? qsTr("Saving…") : qsTr("Save")
                    enabled: !editor.saving && editor.asset !== null
                    onClicked: editor.submit()
                }
            }
        }
    }
}
