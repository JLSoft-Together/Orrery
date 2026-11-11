import 'package:app_links/app_links.dart';

abstract class LinkSource {
  Stream<Uri> get links;
}

class AppLinksSource implements LinkSource {
  const AppLinksSource();

  @override
  Stream<Uri> get links => AppLinks().uriLinkStream;
}
