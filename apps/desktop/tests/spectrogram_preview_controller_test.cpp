#include "spectrogram_preview_controller.hpp"
#include "spectrogram_palette.hpp"
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFile>
#include <QImage>
#include <QTemporaryDir>
#include <QThread>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>

namespace {
void require(bool value,const char* message) {if(!value)throw std::runtime_error(message);}
void settle(SpectrogramPreviewController& controller) {
    QElapsedTimer timeout;timeout.start();
    while(controller.running() && timeout.elapsed()<8000) {
        QCoreApplication::processEvents(QEventLoop::AllEvents,10);QThread::msleep(1);
    }
    require(!controller.running(),"viewport analysis exceeded bounded wait");
}
}
int main(int argc,char** argv) {
    QCoreApplication app(argc,argv);
    QTemporaryDir root;const auto source=root.filePath("stereo.wav");
    { std::ofstream file(source.toStdString(),std::ios::binary);
      auto write=[&](auto value){file.write(reinterpret_cast<const char*>(&value),sizeof(value));};
      file.write("RIFF",4);write(std::uint32_t(192036));file.write("WAVEfmt ",8);write(std::uint32_t(16));write(std::uint16_t(1));write(std::uint16_t(2));write(std::uint32_t(48000));write(std::uint32_t(192000));write(std::uint16_t(4));write(std::uint16_t(16));file.write("data",4);write(std::uint32_t(192000));
      for(unsigned frame=0;frame<48000;++frame){auto sample=static_cast<std::int16_t>(std::sin(frame*0.13)*8000);write(sample);write(static_cast<std::int16_t>(-sample));}
    }
    try {
        double previous=0;
        for(unsigned value=0;value<256;value+=4) {
            const auto c=SpectrogramPalette::color(static_cast<std::uint8_t>(value));
            const auto linear=[](double component){component/=255;return component<=0.04045?component/12.92:std::pow((component+0.055)/1.055,2.4);};
            const double luminance=0.2126*linear(c[0])+0.7152*linear(c[1])+0.0722*linear(c[2]);
            require(luminance>previous,"spectral energy palette reverses brightness");previous=luminance;
        }
        SpectrogramPreviewController controller;
        for(int request=0;request<20;++request)
            controller.requestViewport(source,QString::number(request),0,1000,20,24000,request%2==0,8192,-96,0);
        settle(controller);require(!controller.imageUrl().isEmpty(),"latest viewport did not complete");
        const auto encoded=controller.imageUrl().section(',',1).toLatin1();
        const auto image=QImage::fromData(QByteArray::fromBase64(encoded));
        require(image.width()==1024 && image.height()==384,"viewport dimensions changed");
        require(!controller.scaleImageUrl().isEmpty(),"energy legend missing");
        controller.requestViewport(source,"obsolete",0,1000,20,24000,true,8192,-96,0);
        controller.clear();
        QElapsedTimer timeout;timeout.start();
        while(timeout.elapsed()<400){QCoreApplication::processEvents(QEventLoop::AllEvents,10);QThread::msleep(1);}
        require(!controller.running() && controller.imageUrl().isEmpty(),"cancelled result resurfaced");
        controller.requestViewport(root.filePath("missing.wav"),"missing",0,1000,20,24000,true,8192,-96,0);
        settle(controller);require(controller.imageUrl().isEmpty()&&!controller.errorText().isEmpty(),"missing source retained stale image");
        controller.requestViewport(source,"recovered",0,1000,20,24000,true,8192,-96,0);
        settle(controller);require(!controller.imageUrl().isEmpty()&&controller.errorText().isEmpty(),"viewport failed after error recovery");
        std::cout<<"palette luminance, latest-wins queue, cancellation and recovery passed\n";
    } catch(const std::exception& error){std::cerr<<error.what()<<'\n';return 1;}
}
