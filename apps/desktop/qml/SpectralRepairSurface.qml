//! A complete transient marquee / move / resize gesture, committed on release.
import QtQuick
import EchoDesktop
import "SpectralEditing.js" as Editing

Rectangle {
    id: surface
    required property string imageUrl
    required property int durationMillis
    required property real startRatio
    required property real endRatio
    required property int lowHertz
    required property int highHertz
    required property bool logarithmic
    required property var regions
    required property var selection
    required property int selectedIndex
    required property bool layerEnabled
    required property real progress
    property bool eraseMode: false
    property var gestureRegion: null
    readonly property var activeRegion: gestureRegion || selection
    property real hoverHertz: 0
    property real hoverMillis: 0
    property bool hovered: input.containsMouse
    signal selectionChangedByUser(var value, int index)
    signal regionEdited(int index, var value)
    signal eraseRequested(var value)
    color: "#0b1420"
    clip: true
    implicitHeight: 300

    function xAt(millis) { return (millis/durationMillis-startRatio)/Math.max(0.000001,endRatio-startRatio)*width; }
    function yAt(hertz) { return (1-Editing.frequencyRatio(hertz,lowHertz,highHertz,logarithmic))*height; }
    function timeAt(x) { return Editing.clamp(Math.round((startRatio+Editing.clamp(x/width,0,1)*(endRatio-startRatio))*durationMillis),0,durationMillis); }
    function frequencyAt(y) { return Math.round(Editing.hertzAt(1-y/height,lowHertz,highHertz,logarithmic)); }
    function edgeAt(value,x,y) {
        if (!value) return '';
        const left=xAt(value.startMillis), right=xAt(value.endMillis), top=yAt(value.highHertz), bottom=yAt(value.lowHertz);
        if (x<left-6 || x>right+6 || y<top-6 || y>bottom+6) return '';
        let edge='';
        if (Math.abs(x-left)<6) edge+='left'; else if (Math.abs(x-right)<6) edge+='right';
        if (Math.abs(y-top)<6) edge+='top'; else if (Math.abs(y-bottom)<6) edge+='bottom';
        return edge || 'move';
    }
    Image { anchors.fill: parent; source: surface.imageUrl; fillMode: Image.Stretch; smooth: false }
    Repeater {
        model: [50,100,200,500,1000,2000,5000,10000,20000]
        delegate: Rectangle {
            required property int modelData
            y: surface.yAt(modelData); width: surface.width; height: 1
            visible: modelData>surface.lowHertz && modelData<surface.highHertz
            color: '#16e1edf5'
            Text { x: 5; y: Math.max(2-parent.y, -height-2); text: modelData>=1000 ? (modelData/1000)+' kHz' : modelData+' Hz'; color: '#e1edf5'; style: Text.Outline; styleColor: '#0b1420'; font.pixelSize: 10 }
        }
    }
    Repeater {
        model: surface.regions
        delegate: Rectangle {
            required property var modelData
            required property int index
            x: surface.xAt(modelData.startMillis); y: surface.yAt(modelData.highHertz)
            width: surface.xAt(modelData.endMillis)-x; height: surface.yAt(modelData.lowHertz)-y
            color: surface.layerEnabled ? '#127bb9ff' : '#18777777'
            border.color: surface.layerEnabled ? '#8fbeef' : '#777777'
            visible: index!==surface.selectedIndex || !surface.activeRegion
            Text { x: 5; y: 3; visible: parent.width>52 && parent.height>17; text: (index+1)+' · −'+(modelData.attenuationCentibels/100).toFixed(0)+' dB'; color: '#e5f9ff'; font.pixelSize: 10 }
        }
    }
    Rectangle {
        id: selectedBox
        property var value: surface.activeRegion
        visible: value!==null
        x: value ? surface.xAt(value.startMillis) : 0
        y: value ? surface.yAt(value.highHertz) : 0
        width: value ? surface.xAt(value.endMillis)-x : 0
        height: value ? surface.yAt(value.lowHertz)-y : 0
        color: '#167bb9ff'; border.color: '#edf7ff'; border.width: 2
        Rectangle { anchors.fill: parent; anchors.margins: -1; color: "transparent"; border.color: "#b00b1420"; z: -1 }
        Rectangle {
            visible: selectedBox.value!==null
            x: selectedBox.value ? Math.min(parent.width/2, surface.xAt(selectedBox.value.startMillis+selectedBox.value.timeFeatherMillis)-parent.x) : 0
            y: selectedBox.value ? Math.min(parent.height/2, surface.yAt(selectedBox.value.highHertz-selectedBox.value.frequencyFeatherHertz)-parent.y) : 0
            width: Math.max(0,parent.width-2*x); height: Math.max(0,parent.height-2*y)
            color: 'transparent'; border.color: '#907bb9ff'
        }
        Repeater {
            model: [[0,0],[1,0],[0,1],[1,1]]
            delegate: Rectangle { required property var modelData; x: modelData[0]*selectedBox.width-3; y: modelData[1]*selectedBox.height-3; width: 6; height: 6; color: '#edf7ff'; border.color: '#163a57' }
        }
    }
    Rectangle { x: (surface.progress-surface.startRatio)/Math.max(0.000001,surface.endRatio-surface.startRatio)*parent.width; width: 1; height: parent.height; color: '#f8faff' }
    MouseArea {
        id: input
        objectName: 'spectralPointer'
        anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.LeftButton
        enabled: surface.durationMillis>0
        cursorShape: pressed && operation==='move' ? Qt.ClosedHandCursor : Qt.CrossCursor
        property real originX: 0
        property real originY: 0
        property var originalRegion: null
        property int index: -1
        property string operation: ''
        onPressed: mouse => {
            surface.forceActiveFocus(); originX=mouse.x; originY=mouse.y;
            index=surface.selectedIndex; originalRegion=surface.selection;
            operation=surface.eraseMode || (mouse.modifiers & Qt.AltModifier) ? '' : surface.edgeAt(originalRegion,mouse.x,mouse.y);
            if (!operation && !surface.eraseMode && !(mouse.modifiers & Qt.AltModifier)) {
                for (let i=surface.regions.length-1;i>=0;--i) {
                    if (surface.edgeAt(surface.regions[i],mouse.x,mouse.y)) {
                        index=i; originalRegion=surface.regions[i]; operation='move';
                        surface.selectionChangedByUser(originalRegion,index); break;
                    }
                }
            }
            if (!operation) { operation='draw'; index=-1; originalRegion=null; }
        }
        onPositionChanged: mouse => {
            surface.hoverMillis=surface.timeAt(mouse.x); surface.hoverHertz=surface.frequencyAt(mouse.y);
            if (!pressed) return;
            if (operation==='draw') surface.gestureRegion=Editing.selection(surface.timeAt(originX),surface.timeAt(mouse.x),surface.frequencyAt(originY),surface.frequencyAt(mouse.y),surface.durationMillis);
            else if (operation==='move') surface.gestureRegion=Editing.move(originalRegion,surface.timeAt(mouse.x)-surface.timeAt(originX),surface.frequencyAt(mouse.y)-surface.frequencyAt(originY),surface.durationMillis);
            else surface.gestureRegion=Editing.resize(originalRegion,operation,surface.timeAt(mouse.x),surface.frequencyAt(mouse.y),surface.durationMillis);
        }
        onReleased: mouse => {
            const moved=Math.abs(mouse.x-originX)>=3 || Math.abs(mouse.y-originY)>=3;
            const value=surface.gestureRegion;
            if (value && moved) {
                if (surface.eraseMode) surface.eraseRequested(value);
                else { surface.selectionChangedByUser(value,index); if(index>=0) surface.regionEdited(index,value); }
            }
            surface.gestureRegion=null; operation='';
        }
        onCanceled: { surface.gestureRegion=null; operation=''; }
    }
}
