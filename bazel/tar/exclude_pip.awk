# Appended (via mtree_mutate's `includes`) after tar.bzl's default mutation
# pipeline, so $1 already carries the final in-tar path (package_dir applied).
# python-build-standalone's install_only[_stripped] archive always bundles
# pip/ensurepip/wheel, plus the pip/pip3/pip3.12 console-script shims under
# bin/, even though nothing in this image invokes pip; dropping each whole
# subtree (dir entry plus every descendant) or shim file here, rather than
# only the leaf files, keeps no orphaned directory or dangling entry behind.
$1 ~ /\/lib\/python3\.12\/ensurepip(\/|$)/ { next }
$1 ~ /\/lib\/python3\.12\/site-packages\/pip(-[^\/]*)?(\/|$)/ { next }
$1 ~ /\/lib\/python3\.12\/site-packages\/wheel(-[^\/]*)?(\/|$)/ { next }
$1 ~ /\/bin\/pip(3(\.12)?)?$/ { next }
