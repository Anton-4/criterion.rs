use serialize::json;
use std::fmt::Show;

use analyze;
use bencher::Bencher;
use clock::Clock;
use file;
use fs;
use plot;
use sample::Sample;


pub struct Criterion {
    pub confidence_level: f64,
    pub measurement_time: uint,
    pub nresamples: uint,
    pub sample_size: u64,
    pub warm_up_time: uint,
}

impl Criterion {
    // XXX What would be a good default?
    // XXX Should this be named `new` or `default`?
    pub fn default() -> Criterion {
        Criterion {
            confidence_level: 0.95,
            measurement_time: 10,
            nresamples: 100_000,
            sample_size: 100,
            warm_up_time: 1_000,
        }
    }

    pub fn confidence_level<'a>(&'a mut self, cl: f64) -> &'a mut Criterion {
        self.confidence_level = cl;

        self
    }

    pub fn measurement_time<'a>(&'a mut self, t: uint) -> &'a mut Criterion {
        self.measurement_time = t;

        self
    }

    pub fn nresamples<'a>(&'a mut self, n: uint) -> &'a mut Criterion {
        self.nresamples = n;

        self
    }

    pub fn sample_size<'a>(&'a mut self, n: u64) -> &'a mut Criterion {
        self.sample_size = n;

        self
    }

    pub fn warm_up_time<'a>(&'a mut self, ms: uint) -> &'a mut Criterion {
        self.warm_up_time = ms;

        self
    }

    pub fn bench<'a,
                 N: Str + ToStr>(
                 &'a mut self,
                 name: N,
                 f: |&mut Bencher|)
                 -> &'a mut Criterion {
        local_data_key!(clock: Clock);

        let cl = self.confidence_level;
        let name = name.as_slice();
        let nresamples = self.nresamples;

        if clock.get().is_none() {
            clock.replace(Some(Clock::new(self)));
        }

        println!("\nbenchmarking {}", name);

        let sample = Sample::new(f, self);

        let root_dir = Path::new(".criterion").join(name);
        let new_dir = root_dir.join("new");
        let base_dir = root_dir.join("base");

        if base_dir.exists() { fs::rmrf(&base_dir) }

        if new_dir.exists() { fs::mv(&new_dir, &base_dir) }

        fs::mkdirp(&new_dir);
        sample.save(&new_dir.join("sample.json"));

        plot::pdf(sample.data(), &new_dir);

        let outliers = sample.classify_outliers();
        outliers.save(&new_dir.join("outliers"));
        outliers.report();

        plot::outliers(&outliers, &new_dir.join("outliers"));

        let sample = outliers.normal();
        let (estimates, distributions) =
            analyze::estimate_statistics(sample, nresamples, cl);

        let bootstrap_dir = new_dir.join("bootstrap");
        let distributions_dir = bootstrap_dir.join("distributions");
        fs::mkdirp(&distributions_dir);

        plot::distribution(distributions.get(0).as_slice(),
                           estimates.get(&"mean"),
                           &distributions_dir,
                           "mean");

        plot::distribution(distributions.get(1).as_slice(),
                           estimates.get(&"median"),
                           &distributions_dir,
                           "median");

        plot::distribution(distributions.get(2).as_slice(),
                           estimates.get(&"SD"),
                           &distributions_dir,
                           "SD");

        plot::distribution(distributions.get(3).as_slice(),
                           estimates.get(&"MAD"),
                           &distributions_dir,
                           "MAD");

        file::write(&new_dir.join("bootstrap/estimates.json"),
                    json::encode(&estimates).as_slice());

        if !base_dir.exists() {
            return self;
        }

        let base_sample = Sample::load(&base_dir.join("sample.json"));
        let base_sample = base_sample.data();

        plot::both_points(base_sample, sample, &root_dir.join("both"));
        plot::both_pdfs(base_sample, sample, &root_dir.join("both"));

        let (estimates, _) =
            analyze::compare_samples(base_sample, sample, nresamples, cl);

        // TODO Plot the bootstrap distribution of the change
        //plot::bootstraps(&distributions, &root_dir.join("change"));

        let change_dir = root_dir.join("change");
        fs::mkdirp(&change_dir.join("bootstrap"));
        file::write(&change_dir.join("bootstrap/estimates.json"),
                    json::encode(&estimates).as_slice());

        self
    }

    pub fn bench_group<'a,
                       G: Show,
                       I: Clone + Show>(
                       &'a mut self,
                       group: G,
                       inputs: &[I],
                       f: |&mut Bencher, I|)
                       -> &'a mut Criterion {
        for input in inputs.iter() {
            self.bench(format!("{}/{}", group, input), |x| {
                f(x, input.clone())
            });
        }

        // TODO Summary analysis

        self
    }
}
