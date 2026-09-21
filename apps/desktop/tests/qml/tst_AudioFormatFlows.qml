import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id:test;name:"AudioFormatFlows";width:800;height:800;visible:true;when:windowShown
    property var admitted:[]
    QtObject {
        id:audioStreamImport
        property bool busy:false;property bool choosing:false;property var streams:[]
        property string sourceName:"video.mkv";property string errorText:""
        property int picked:-1
        signal stateChanged();signal filesReady(var files)
        function inspect(files){streams=[{index:1,title:"Original",language:"en",codec:"aac",sampleRate:48000,channels:2},{index:3,title:"Commentary",language:"zh",codec:"aac",sampleRate:48000,channels:2}];choosing=true;stateChanged();}
        function select(index){picked=index;busy=true;choosing=false;stateChanged();}
        function cancel(){busy=false;choosing=false;streams=[];stateChanged();}
    }
    AudioExportSettings { id:settings;width:500 }
    AudioStreamImportDialog { id:intake;onFilesReady:files=>test.admitted=files }
    function init(){intake.close();admitted=[];audioStreamImport.cancel();audioStreamImport.picked=-1;findChild(settings,"exportFormat").currentIndex=0;findChild(settings,"exportSampleRate").currentIndex=1;findChild(settings,"exportChannels").currentIndex=1;}
    function test_delivery_options_are_consistent(){
        compare(settings.options.format,"wav_pcm24");compare(settings.options.sampleRate,48000);compare(settings.options.channels,2);
        findChild(settings,"exportFormat").currentIndex=2;findChild(settings,"exportSampleRate").currentIndex=2;findChild(settings,"exportChannels").currentIndex=0;
        compare(settings.options.format,"wav_float32");compare(settings.options.sampleRate,96000);compare(settings.options.channels,1);
        findChild(settings,"exportFormat").currentIndex=4;tryCompare(settings,"compressed",true);verify(settings.options.sampleRate===44100||settings.options.sampleRate===48000);compare(settings.extension,"mp3");
        findChild(settings,"exportBitrate").currentIndex=3;compare(settings.options.bitrateKbps,320);
        findChild(settings,"exportFormat").currentIndex=5;compare(settings.extension,"m4a");
        verify(settings.implicitHeight<350);
    }
    function test_switching_format_preserves_supported_sample_rate(){
        settings.configure({format:"wav_pcm24",sampleRate:48000,channels:2});
        findChild(settings,"exportFormat").currentIndex=5;
        tryCompare(findChild(settings,"exportSampleRate"),"currentIndex",1);
        compare(settings.options.sampleRate,48000);
        settings.configure({format:"wav_float32",sampleRate:96000,channels:1});
        findChild(settings,"exportFormat").currentIndex=4;
        tryCompare(findChild(settings,"exportSampleRate"),"currentIndex",1);
        compare(settings.options.sampleRate,48000);
    }
    function test_stream_choice_is_explicit_and_durable_files_survive_close(){
        intake.present(["file:///test/video.mkv"]);tryCompare(intake,"opened",true);
        const button=findChild(intake,"importSelectedStream");verify(!button.enabled);intake.selectedStream=3;verify(button.enabled);mouseClick(button);compare(audioStreamImport.picked,3);verify(!button.enabled);
        audioStreamImport.filesReady(["file:///test/media/container-audio/track-3.mka"]);tryCompare(intake,"visible",false);compare(admitted,["file:///test/media/container-audio/track-3.mka"]);
        audioStreamImport.filesReady(["file:///late.mka"]);compare(admitted.length,1);compare(admitted[0],"file:///test/media/container-audio/track-3.mka");
    }
}
