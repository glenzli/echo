//! Desktop backend facade: the only Qt-owned bridge to the Rust memory
//! engine. QML never opens SQLite; all Library reads go through this object.

#pragma once

#include <QObject>
#include <QVariantList>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

class DesktopBackend : public QObject {
    Q_OBJECT
    Q_PROPERTY(quint64 assetCount READ assetCount NOTIFY assetsChanged)
    Q_PROPERTY(QString catalogPath READ catalogPath NOTIFY assetsChanged)

  public:
    explicit DesktopBackend(
        rust::Box<echo::desktop::LibrarySession> session,
        QObject* parent = nullptr
    );

    Q_INVOKABLE void refresh();
    Q_INVOKABLE QVariantList listAssets() const;
    quint64 assetCount() const;
    QString catalogPath() const;

  signals:
    void assetsChanged();

  private:
    rust::Box<echo::desktop::LibrarySession> session_;
};
