(function() {var type_impls = {
"ext":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BoundedChainComplex-for-FiniteChainComplex%3CM,+F%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#174-182\">source</a><a href=\"#impl-BoundedChainComplex-for-FiniteChainComplex%3CM,+F%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;M, F&gt; <a class=\"trait\" href=\"ext/chain_complex/trait.BoundedChainComplex.html\" title=\"trait ext::chain_complex::BoundedChainComplex\">BoundedChainComplex</a> for <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;M, F&gt;<div class=\"where\">where\n    M: Module,\n    F: ModuleHomomorphism&lt;Source = M, Target = M&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.max_s\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#179-181\">source</a><a href=\"#method.max_s\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.BoundedChainComplex.html#tymethod.max_s\" class=\"fn\">max_s</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.u32.html\">u32</a></h4></section><section id=\"method.euler_characteristic\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/mod.rs.html#299-303\">source</a><a href=\"#method.euler_characteristic\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.BoundedChainComplex.html#method.euler_characteristic\" class=\"fn\">euler_characteristic</a>(&amp;self, t: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.i32.html\">i32</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.isize.html\">isize</a></h4></section></div></details>","BoundedChainComplex","ext::CCC"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-ChainComplex-for-FiniteChainComplex%3CM,+F%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#123-172\">source</a><a href=\"#impl-ChainComplex-for-FiniteChainComplex%3CM,+F%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;M, F&gt; <a class=\"trait\" href=\"ext/chain_complex/trait.ChainComplex.html\" title=\"trait ext::chain_complex::ChainComplex\">ChainComplex</a> for <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;M, F&gt;<div class=\"where\">where\n    M: Module,\n    F: ModuleHomomorphism&lt;Source = M, Target = M&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"associatedtype.Algebra\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Algebra\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Algebra\" class=\"associatedtype\">Algebra</a> = &lt;M as Module&gt;::Algebra</h4></section><section id=\"associatedtype.Homomorphism\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Homomorphism\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Homomorphism\" class=\"associatedtype\">Homomorphism</a> = F</h4></section><section id=\"associatedtype.Module\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Module\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Module\" class=\"associatedtype\">Module</a> = M</h4></section><section id=\"method.algebra\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#132-134\">source</a><a href=\"#method.algebra\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.algebra\" class=\"fn\">algebra</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;Self::<a class=\"associatedtype\" href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Algebra\" title=\"type ext::chain_complex::ChainComplex::Algebra\">Algebra</a>&gt;</h4></section><section id=\"method.min_degree\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#136-138\">source</a><a href=\"#method.min_degree\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.min_degree\" class=\"fn\">min_degree</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.i32.html\">i32</a></h4></section><section id=\"method.zero_module\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#140-142\">source</a><a href=\"#method.zero_module\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.zero_module\" class=\"fn\">zero_module</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;Self::<a class=\"associatedtype\" href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Module\" title=\"type ext::chain_complex::ChainComplex::Module\">Module</a>&gt;</h4></section><section id=\"method.module\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#144-151\">source</a><a href=\"#method.module\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.module\" class=\"fn\">module</a>(&amp;self, s: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.u32.html\">u32</a>) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;Self::<a class=\"associatedtype\" href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Module\" title=\"type ext::chain_complex::ChainComplex::Module\">Module</a>&gt;</h4></section><details class=\"toggle method-toggle\" open><summary><section id=\"method.differential\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#153-157\">source</a><a href=\"#method.differential\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.differential\" class=\"fn\">differential</a>(&amp;self, s: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.u32.html\">u32</a>) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;Self::<a class=\"associatedtype\" href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Homomorphism\" title=\"type ext::chain_complex::ChainComplex::Homomorphism\">Homomorphism</a>&gt;</h4></section></summary><div class='docblock'>This returns the differential starting from the sth module.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.compute_through_bidegree\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#159-163\">source</a><a href=\"#method.compute_through_bidegree\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.compute_through_bidegree\" class=\"fn\">compute_through_bidegree</a>(&amp;self, b: Bidegree)</h4></section></summary><div class='docblock'>Ensure all bidegrees less than or equal to (s, t) have been computed</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.has_computed_bidegree\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#165-167\">source</a><a href=\"#method.has_computed_bidegree\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.has_computed_bidegree\" class=\"fn\">has_computed_bidegree</a>(&amp;self, b: Bidegree) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>If the complex has been computed at bidegree (s, t). This means the module has been\ncomputed at (s, t), and so has the differential at (s, t). In the case of a free module,\nthe target of the differential, namely the bidegree (s - 1, t), need not be computed, as\nlong as all the generators hit by the differential have already been computed.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.next_homological_degree\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#169-171\">source</a><a href=\"#method.next_homological_degree\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#tymethod.next_homological_degree\" class=\"fn\">next_homological_degree</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.u32.html\">u32</a></h4></section></summary><div class='docblock'>The first s such that <code>self.module(s)</code> is not defined.</div></details><section id=\"method.prime\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/mod.rs.html#166-168\">source</a><a href=\"#method.prime\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#method.prime\" class=\"fn\">prime</a>(&amp;self) -&gt; ValidPrime</h4></section><details class=\"toggle method-toggle\" open><summary><section id=\"method.iter_stem\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/mod.rs.html#192-198\">source</a><a href=\"#method.iter_stem\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#method.iter_stem\" class=\"fn\">iter_stem</a>(&amp;self) -&gt; <a class=\"struct\" href=\"ext/chain_complex/struct.StemIterator.html\" title=\"struct ext::chain_complex::StemIterator\">StemIterator</a>&lt;'_, Self&gt; <a href=\"#\" class=\"tooltip\" data-notable-ty=\"StemIterator&lt;&#39;_, Self&gt;\">ⓘ</a></h4></section></summary><div class='docblock'>Iterate through all defined bidegrees in increasing order of stem. The return values are of\nthe form <code>(s, n, t)</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.apply_quasi_inverse\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/mod.rs.html#206-227\">source</a><a href=\"#method.apply_quasi_inverse\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#method.apply_quasi_inverse\" class=\"fn\">apply_quasi_inverse</a>&lt;T, S&gt;(\n    &amp;self,\n    results: &amp;mut <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.slice.html\">[T]</a>,\n    b: Bidegree,\n    inputs: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.slice.html\">[S]</a>,\n) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.bool.html\">bool</a><div class=\"where\">where\n    for&lt;'a&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.reference.html\">&amp;'a mut T</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/convert/trait.Into.html\" title=\"trait core::convert::Into\">Into</a>&lt;SliceMut&lt;'a&gt;&gt;,\n    for&lt;'a&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.reference.html\">&amp;'a S</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/convert/trait.Into.html\" title=\"trait core::convert::Into\">Into</a>&lt;Slice&lt;'a&gt;&gt;,</div></h4></section></summary><div class='docblock'>Apply the quasi-inverse of the (s, t)th differential to the list of inputs and results.\nThis defaults to applying <code>self.differentials(s).quasi_inverse(t)</code>, but in some cases\nthe quasi-inverse might be stored separately on disk. <a href=\"ext/chain_complex/trait.ChainComplex.html#method.apply_quasi_inverse\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.save_dir\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/mod.rs.html#230-232\">source</a><a href=\"#method.save_dir\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#method.save_dir\" class=\"fn\">save_dir</a>(&amp;self) -&gt; &amp;<a class=\"enum\" href=\"ext/save/enum.SaveDirectory.html\" title=\"enum ext::save::SaveDirectory\">SaveDirectory</a></h4></section></summary><div class='docblock'>A directory used to save information about the chain complex.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.save_file\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/mod.rs.html#235-246\">source</a><a href=\"#method.save_file\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"ext/chain_complex/trait.ChainComplex.html#method.save_file\" class=\"fn\">save_file</a>(&amp;self, kind: <a class=\"enum\" href=\"ext/save/enum.SaveKind.html\" title=\"enum ext::save::SaveKind\">SaveKind</a>, b: Bidegree) -&gt; <a class=\"struct\" href=\"ext/save/struct.SaveFile.html\" title=\"struct ext::save::SaveFile\">SaveFile</a>&lt;Self::<a class=\"associatedtype\" href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Algebra\" title=\"type ext::chain_complex::ChainComplex::Algebra\">Algebra</a>&gt;</h4></section></summary><div class='docblock'>Get the save file of a bidegree</div></details></div></details>","ChainComplex","ext::CCC"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-FiniteChainComplex%3CM%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#86-121\">source</a><a href=\"#impl-FiniteChainComplex%3CM%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;M: Module&gt; <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;M, FullModuleHomomorphism&lt;M&gt;&gt;</h3></section></summary><div class=\"impl-items\"><section id=\"method.map\" class=\"method\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#87-120\">source</a><h4 class=\"code-header\">pub fn <a href=\"ext/chain_complex/struct.FiniteChainComplex.html#tymethod.map\" class=\"fn\">map</a>&lt;N: Module&lt;Algebra = M::Algebra&gt;&gt;(\n    &amp;self,\n    f: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/ops/function/trait.FnMut.html\" title=\"trait core::ops::function::FnMut\">FnMut</a>(<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.reference.html\">&amp;M</a>) -&gt; N,\n) -&gt; <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;N, FullModuleHomomorphism&lt;N&gt;&gt;</h4></section></div></details>",0,"ext::CCC"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-FiniteChainComplex%3CM,+F%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#21-55\">source</a><a href=\"#impl-FiniteChainComplex%3CM,+F%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;M, F&gt; <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;M, F&gt;<div class=\"where\">where\n    M: Module + ZeroModule,\n    F: ModuleHomomorphism&lt;Source = M, Target = M&gt; + ZeroHomomorphism&lt;M, M&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#26-50\">source</a><h4 class=\"code-header\">pub fn <a href=\"ext/chain_complex/struct.FiniteChainComplex.html#tymethod.new\" class=\"fn\">new</a>(modules: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;M&gt;&gt;, differentials: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;F&gt;&gt;) -&gt; Self</h4></section><section id=\"method.ccdz\" class=\"method\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#52-54\">source</a><h4 class=\"code-header\">pub fn <a href=\"ext/chain_complex/struct.FiniteChainComplex.html#tymethod.ccdz\" class=\"fn\">ccdz</a>(module: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;M&gt;) -&gt; Self</h4></section></div></details>",0,"ext::CCC"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-FiniteChainComplex%3CM,+F%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#57-84\">source</a><a href=\"#impl-FiniteChainComplex%3CM,+F%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;M, F&gt; <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;M, F&gt;<div class=\"where\">where\n    M: Module,\n    F: ModuleHomomorphism&lt;Source = M, Target = M&gt; + ZeroHomomorphism&lt;M, M&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.pop\" class=\"method\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#62-83\">source</a><h4 class=\"code-header\">pub fn <a href=\"ext/chain_complex/struct.FiniteChainComplex.html#tymethod.pop\" class=\"fn\">pop</a>(&amp;mut self)</h4></section></div></details>",0,"ext::CCC"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-FiniteChainComplex%3CM,+F1%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#316-335\">source</a><a href=\"#impl-FiniteChainComplex%3CM,+F1%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;M, F1&gt; <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;M, F1&gt;<div class=\"where\">where\n    M: Module,\n    F1: ModuleHomomorphism&lt;Source = M, Target = M&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.augment\" class=\"method\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#321-334\">source</a><h4 class=\"code-header\">pub fn <a href=\"ext/chain_complex/struct.FiniteChainComplex.html#tymethod.augment\" class=\"fn\">augment</a>&lt;CC: <a class=\"trait\" href=\"ext/chain_complex/trait.ChainComplex.html\" title=\"trait ext::chain_complex::ChainComplex\">ChainComplex</a>&lt;Algebra = M::Algebra&gt;, F2: ModuleHomomorphism&lt;Source = M, Target = CC::<a class=\"associatedtype\" href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Module\" title=\"type ext::chain_complex::ChainComplex::Module\">Module</a>&gt;&gt;(\n    self,\n    target_cc: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;CC&gt;,\n    chain_maps: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;F2&gt;&gt;,\n) -&gt; <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteAugmentedChainComplex.html\" title=\"struct ext::chain_complex::FiniteAugmentedChainComplex\">FiniteAugmentedChainComplex</a>&lt;M, F1, F2, CC&gt;</h4></section></div></details>",0,"ext::CCC"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CFiniteAugmentedChainComplex%3CM,+F1,+F2,+CC%3E%3E-for-FiniteChainComplex%3CM,+F1%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#292-302\">source</a><a href=\"#impl-From%3CFiniteAugmentedChainComplex%3CM,+F1,+F2,+CC%3E%3E-for-FiniteChainComplex%3CM,+F1%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;M, F1, F2, CC&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"ext/chain_complex/struct.FiniteAugmentedChainComplex.html\" title=\"struct ext::chain_complex::FiniteAugmentedChainComplex\">FiniteAugmentedChainComplex</a>&lt;M, F1, F2, CC&gt;&gt; for <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteChainComplex.html\" title=\"struct ext::chain_complex::FiniteChainComplex\">FiniteChainComplex</a>&lt;M, F1&gt;<div class=\"where\">where\n    M: Module,\n    CC: <a class=\"trait\" href=\"ext/chain_complex/trait.ChainComplex.html\" title=\"trait ext::chain_complex::ChainComplex\">ChainComplex</a>&lt;Algebra = M::Algebra&gt;,\n    F1: ModuleHomomorphism&lt;Source = M, Target = M&gt;,\n    F2: ModuleHomomorphism&lt;Source = M, Target = CC::<a class=\"associatedtype\" href=\"ext/chain_complex/trait.ChainComplex.html#associatedtype.Module\" title=\"type ext::chain_complex::ChainComplex::Module\">Module</a>&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/ext/chain_complex/finite_chain_complex.rs.html#299-301\">source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.80.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(c: <a class=\"struct\" href=\"ext/chain_complex/struct.FiniteAugmentedChainComplex.html\" title=\"struct ext::chain_complex::FiniteAugmentedChainComplex\">FiniteAugmentedChainComplex</a>&lt;M, F1, F2, CC&gt;) -&gt; Self</h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<FiniteAugmentedChainComplex<M, F1, F2, CC>>","ext::CCC"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()