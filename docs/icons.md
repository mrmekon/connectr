# Regenerating icon files

The Connectr menu bar icon is saved in Gimp's XCF format, as _connectr.xcf_.

The OS X icons are generated from the XCF by resizing it down to 80px with a 300 DPI and saving it as a PNG, _connectr_80px_300dpi.png_.  OS X does actually use the DPI, so setting it is critical to getting sharp rendering at the right size on Retina displays.  The menu bar itself is 22px high, so a 20px @ 72dpi icon looks ideal.  To handle Retina displays, this means bumping to 40px @ 150dpi.  However, you can keep doubling, and I find 80px @ 300dpi to be nicer to work with, so I went with that.

The Windows icon is generated from the OS X PNG, mostly because Gimp is an idiot and doesn't like exporting XCF->ICO directly.  Just open _connectr_80px_300dpi.png_ and export it as _connectr.ico_ without any changes.

