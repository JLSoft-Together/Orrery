import 'package:app_links/app_links.dart';

abstract class LinkSource {
  Stream<Uri> get links;
}

class MergedLinkSource implements LinkSource {
  const MergedLinkSource(this.sources);

  final List<LinkSource> sources;

  @override
  Stream<Uri> get links => Stream.multi((controller) {
    final subs = [
      for (final source in sources)
        source.links.listen(controller.add, onError: (Object _) {}),
    ];
    controller.onCancel = () => Future.wait([
      for (final sub in subs) sub.cancel(),
    ]);
  });
}

class AppLinksSource implements LinkSource {
  const AppLinksSource();

  @override
  Stream<Uri> get links => AppLinks().uriLinkStream;
}
