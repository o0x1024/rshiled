use std::{cell::RefCell, collections::VecDeque, future::Future, pin::Pin, rc::Rc, time::Duration};

use boa_engine::{
    context::ContextBuilder,
    job::{FutureJob, JobQueue, NativeJob},
    js_string,
    property::Attribute,
    Context, JsArgs, JsResult, JsValue, NativeFunction, Source,
};
use boa_runtime::Console;

use tokio::task::{self, JoinSet, LocalSet};

struct MyJobQueue {
    futures: RefCell<JoinSet<NativeJob>>,
    jobs: RefCell<VecDeque<NativeJob>>,
}

impl MyJobQueue {
    fn new() -> Self {
        Self {
            futures: RefCell::default(),
            jobs: RefCell::default(),
        }
    }
}

impl JobQueue for MyJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob, _context: &mut Context) {
        self.jobs.borrow_mut().push_back(job);
    }

    fn enqueue_future_job(&self, future: FutureJob, _context: &mut Context) {
        self.futures.borrow_mut().spawn_local(future);
    }

    fn run_jobs(&self, context: &mut Context) {
        let mut next_job = self.jobs.borrow_mut().pop_front();
        while let Some(job) = next_job {
            if job.call(context).is_err() {
                self.jobs.borrow_mut().clear();
                return;
            };
            next_job = self.jobs.borrow_mut().pop_front();
        }
    }
    fn run_jobs_async<'a, 'ctx, 'fut>(
        &'a self,
        context: &'ctx mut Context,
    ) -> Pin<Box<dyn Future<Output = ()> + 'fut>>
    where
        'a: 'fut,
        'ctx: 'fut,
    {
        Box::pin(async {
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async {
                    while !(self.jobs.borrow().is_empty() && self.futures.borrow().is_empty()) {
                        context.run_jobs();

                        if let Some(res) = self.futures.borrow_mut().join_next().await {
                            context.enqueue_job(res.unwrap())
                        }
                    }
                })
                .await;
        })
    }
}

fn fetch_get(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> impl Future<Output = JsResult<JsValue>> {
    let client = reqwest::Client::new();
    let url = args
        .get_or_undefined(0)
        .to_string(context)
        .unwrap()
        .to_std_string()
        .unwrap();

    async move {
        // tokio::time::sleep(Duration::from_secs(2)).await;
        let response = client.get(url).send().await.unwrap();
        let body = response.text().await.unwrap();
        // println!("{}",body);
        Ok(JsValue::new(js_string!(body)))
    }
}

#[tauri::command]
pub fn test_javascript(script: String) -> Result<String, String> {
    let mut context = Context::default();
    let console = Console::init(&mut context);

    context
        .register_global_property(js_string!(Console::NAME), console, Attribute::all())
        .expect("the console object shouldn't exist yet");
    context
        .register_global_builtin_callable(
            js_string!("fetch_get"),
            1,
            NativeFunction::from_async_fn(fetch_get),
        )
        .expect("the sleep builtin shouldn't exist yet");

    match context.eval(Source::from_bytes(&script)) {
        Ok(res) => {
            println!(
                "rust:{}",
                res.to_string(&mut context).unwrap().to_std_string_escaped()
            );
            Ok(res.to_string(&mut context).unwrap().to_std_string_escaped())
        }
        Err(e) => {
            // Pretty print the error
            eprintln!("Uncaught {e}");
            Err(e.to_string())
        }
    }
}

// #[tauri::command(async)]
// pub async fn test_javascript(script: String) -> Result<String, String> {
//     let result = task::spawn_blocking(||  {

//         let queue = MyJobQueue::new();

//         let mut context = &mut ContextBuilder::new()
//             .job_queue(Rc::new(queue))
//             .build()
//             .unwrap();

//         let console = Console::init(&mut context);

//         context
//             .register_global_property(js_string!(Console::NAME), console, Attribute::all())
//             .expect("the console object shouldn't exist yet");

//         context
//             .register_global_builtin_callable(
//                 js_string!("fetch_get"),
//                 1,
//                 NativeFunction::from_async_fn(fetch_get),
//             )
//             .expect("the sleep builtin shouldn't exist yet");

//         let job = NativeJob::new(move |context| -> JsResult<JsValue> {
//             let result = context.eval(Source::from_bytes(&script))?;
//             Ok(result)
//         });

//         context.enqueue_job(job);

//         tokio::spawn(async move {
//         context.run_jobs_async().await;
// });
//         // context.run_jobs_async().await;
//     })
//     .await
//     .unwrap(); // 等待任务完成并获取结果
//     println!("{:?}", result);
//     Ok("".to_string())
//     // tokio::spawn(async move {

//     // });
// }
