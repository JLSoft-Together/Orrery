import '../remote/protocol.dart';
import '../ui/viewers/viewer_api.dart';
import 'prompt_model.dart';

class PromptsApi {
  PromptsApi(this.viewer);

  final ViewerApi viewer;

  Stream<JsonMap> get events => viewer.events;

  Future<PromptList> list(String? paneKey) async {
    final data = await viewer.map('prompts.list', {'paneKey': ?paneKey});
    return PromptList.parse(data);
  }

  Future<void> save(PromptDraft draft) async {
    _ensureDone(await viewer.call('prompts.save', draft.toFields()));
  }

  Future<void> delete(String id) async {
    _ensureDone(await viewer.call('prompts.delete', {'promptId': id}));
  }

  void _ensureDone(ViewerReply reply) {
    if (reply.done) return;
    throw ViewerFailure(reply.cancelled ? 'cancelled' : 'pending');
  }
}
