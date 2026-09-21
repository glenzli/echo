//! Compact post-fader stereo peak display. Values are producer-side dBFS.
import QtQuick
import QtQuick.Controls

Item {
    id: meter
    property real leftDb: -70
    property real rightDb: -70
    readonly property real peakDb: Math.max(leftDb, rightDb)
    implicitWidth: 108
    implicitHeight: 18
    Accessible.role: Accessible.Indicator
    Accessible.name: qsTr("Track level")
    Accessible.description: peakDb > -60 ? peakDb.toFixed(1) + " dBFS" : qsTr("Silent")

    Column {
        anchors.left: parent.left
        anchors.right: reading.left
        anchors.rightMargin: 5
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2
        Repeater {
            model: 2
            Rectangle {
                required property int index
                readonly property real sampleDb: index === 0 ? meter.leftDb : meter.rightDb
                width: parent.width
                height: 4
                radius: 1
                color: Theme.track
                Rectangle {
                    width: parent.width * Math.max(0, Math.min(1, (parent.sampleDb + 60) / 60))
                    height: parent.height
                    radius: 1
                    color: parent.sampleDb >= 0 ? Theme.likeAccent : parent.sampleDb > -6 ? Theme.ratingAccent : Theme.accent
                }
                Rectangle {
                    x: parent.width * 0.9
                    width: 1
                    height: parent.height
                    color: Theme.panel
                }
            }
        }
    }
    Text {
        id: reading
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        width: 31
        text: meter.peakDb > -60 ? meter.peakDb.toFixed(0) : "−∞"
        font.family: "Menlo"
        font.pixelSize: 9
        horizontalAlignment: Text.AlignRight
        color: meter.peakDb >= 0 ? Theme.likeAccent : Theme.textSecondary
    }
    HoverHandler { id: hover }
    ToolTip.visible: hover.hovered
    ToolTip.text: qsTr("Stereo peak after track gain, before master processing. Preview buffering may lead audible output.")
}
