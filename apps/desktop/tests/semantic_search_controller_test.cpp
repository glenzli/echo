//! Async presentation contract with a controllable slow inference boundary.
#include "semantic_search_controller.hpp"
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QSemaphore>
#include <QThread>
#include <QThreadPool>
#include <QVariantMap>
#include <atomic>
#include <iostream>
#include <stdexcept>

namespace {
void require(bool condition, const char* message) {
    if (!condition) throw std::runtime_error(message);
}
template<class F> void waitFor(F ready) {
    QElapsedTimer timer; timer.start();
    while (!ready()) {
        require(timer.elapsed() < 5000, "async search timed out");
        QCoreApplication::processEvents(); QThread::msleep(1);
    }
}
}
int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QSemaphore release;
    std::atomic<int> calls{0};
    SemanticSearchController controller("catalog", "online", nullptr,
        [&](const QString&, const QString& endpoint, const QString& query) {
            ++calls;
            if(query == "blocked") release.acquire();
            if(endpoint == "offline") throw std::runtime_error("unavailable");
            return QVariantList{QVariantMap{{"id", query}}};
        });
    try {
        controller.request("blocked"); waitFor([&]{return calls.load() == 1;});
        for(int i=0;i<100;++i) controller.request(QString::number(i));
        QCoreApplication::processEvents();
        require(calls.load() == 1, "superseded searches started competing workers");
        release.release(); waitFor([&]{return !controller.running();});
        require(calls.load() == 2 && controller.resultsQuery() == "99", "latest search did not replace obsolete requests");
        require(controller.results().first().toMap().value("id") == "99", "stale hits escaped into presentation");
        controller.request("blocked"); waitFor([&]{return calls.load() == 3;});
        controller.request("discard this"); controller.clear(); release.release();
        QThreadPool::globalInstance()->waitForDone(); QCoreApplication::processEvents();
        require(!controller.running() && controller.results().isEmpty() && controller.resultsQuery().isEmpty() && calls.load() == 3, "clear restarted a cancelled query or exposed stale results");
        controller.setRuntimeEndpoint("offline"); controller.request("failure"); waitFor([&]{return !controller.running();});
        require(!controller.errorText().isEmpty() && controller.results().isEmpty(), "offline search failed without usable state");
        controller.setRuntimeEndpoint("online"); controller.request("recovered"); waitFor([&]{return !controller.running();});
        require(controller.errorText().isEmpty() && controller.resultsQuery() == "recovered", "search failed to recover after endpoint change");
        std::cout << "100 superseded queries, clear, offline failure and recovery passed\n";
        return 0;
    } catch(const std::exception& e) {
        release.release(10); QThreadPool::globalInstance()->waitForDone();
        std::cerr << e.what() << '\n'; return 1;
    }
}
