# Screenshots

Copy a saved database to a running simulator:

```
alias myapp="xcrun simctl get_app_container booted com.aperturebeam.epsilon data"
cp src/app/stem/db_full/screenshots/* $(myapp)/Documents/
```

Set status bar:

```
xcrun simctl status_bar booted override --time "10:41" --batteryState charged --batteryLevel 100 --cellularBars 4
```
