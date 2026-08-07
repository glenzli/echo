//! Positions the native traffic-light buttons at a fixed distance below the
//! window top so they share the toolbar row with the actions (the QML scene
//! starts below the native title strip, so a simple vertical offset places
//! the lights in the toolbar band).

#import <AppKit/AppKit.h>

#include "mac_titlebar.hpp"

#include <QEvent>
#include <QGuiApplication>
#include <QObject>
#include <QPointer>
#include <QTimer>
#include <QWindow>

#include <array>

namespace {

class MacTitleBarAlignment final : public QObject {
  public:
    MacTitleBarAlignment(QWindow* window, const int traffic_light_center_y) :
        QObject(window), window_(window), traffic_light_center_y_(traffic_light_center_y) {
        window_->installEventFilter(this);
        connect(window_, &QWindow::screenChanged, this, [this] { scheduleAlignment(); });
        scheduleAlignment();
    }

  protected:
    bool eventFilter(QObject* watched, QEvent* event) override {
        if (watched == window_) {
            switch (event->type()) {
            case QEvent::Show:
            case QEvent::Resize:
            case QEvent::WindowStateChange:
                scheduleAlignment();
                break;
            default:
                break;
            }
        }
        return QObject::eventFilter(watched, event);
    }

  private:
    void scheduleAlignment() {
        if (alignment_pending_) {
            return;
        }
        alignment_pending_ = true;
        QTimer::singleShot(0, this, [this] {
            alignment_pending_ = false;
            alignNativeButtons();
        });
    }

    void alignNativeButtons() const {
        if (!window_ || !window_->isVisible() || window_->visibility() == QWindow::FullScreen) {
            return;
        }
        NSView* const qt_view = reinterpret_cast<NSView*>(window_->winId());
        NSWindow* const native_window = qt_view.window;
        if (native_window == nil) {
            return;
        }
        const NSRect window_frame = native_window.frame;
        const NSPoint target_in_screen = NSMakePoint(
            NSMidX(window_frame),
            NSMaxY(window_frame) - static_cast<CGFloat>(traffic_light_center_y_)
        );
        const NSPoint target_in_window = [native_window convertPointFromScreen:target_in_screen];

        constexpr std::array<NSWindowButton, 3> button_types = {
            NSWindowCloseButton,
            NSWindowMiniaturizeButton,
            NSWindowZoomButton,
        };
        for (const NSWindowButton button_type : button_types) {
            NSButton* const button = [native_window standardWindowButton:button_type];
            NSView* const container = button.superview;
            if (button == nil || container == nil) {
                std::fprintf(
                    stderr,
                    "[mac-titlebar] button %ld missing\n",
                    static_cast<long>(button_type)
                );
                continue;
            }
            const NSPoint target_in_container = [container convertPoint:target_in_window
                                                               fromView:nil];
            NSRect frame = button.frame;
            frame.origin.y = target_in_container.y - NSHeight(frame) / 2.0;
            [button setFrameOrigin:frame.origin];
        }
    }

    QPointer<QWindow> window_;
    int traffic_light_center_y_ = 0;
    bool alignment_pending_ = false;
};

} // namespace

void installMacTitleBarAlignment(QWindow* window, const int traffic_light_center_y) {
    if (window == nullptr || traffic_light_center_y <= 0
        || QGuiApplication::platformName() != QStringLiteral("cocoa")) {
        return;
    }
    new MacTitleBarAlignment(window, traffic_light_center_y);
}
