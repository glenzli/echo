//! Millisecond storage with a readable, editable seconds projection.
import QtQuick
import QtQuick.Controls

EchoValueSpinBox {
    id: control
    from: 0
    to: 14400000
    stepSize: 10
    editable: true
    textFromValue: (value, locale) => Number(value / 1000).toLocaleString(locale, "f", 3)
    valueFromText: (text, locale) => Math.round(Number.fromLocaleString(locale, text) * 1000)
    validator: DoubleValidator {
        bottom: control.from / 1000
        top: control.to / 1000
        decimals: 3
        notation: DoubleValidator.StandardNotation
        locale: control.locale.name
    }
}
