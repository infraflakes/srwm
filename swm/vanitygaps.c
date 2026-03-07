/* Helpers for gap functions */
static void
setgaps(int oh, int ov, int ih, int iv)
{
  selmon->gappoh = oh;
  selmon->gappov = ov;
  selmon->gappih = ih;
  selmon->gappiv = iv;
  arrange(selmon);
}

static void
togglegaps(const Arg *arg)
{
  selmon->gappoh = selmon->gappov = !selmon->gappoh;
  selmon->gappih = selmon->gappiv = !selmon->gappih;
  arrange(selmon);
}

static void
defaultgaps(const Arg *arg)
{
  selmon->gappoh = gappoh;
  selmon->gappov = gappov;
  selmon->gappih = gappih;
  selmon->gappiv = gappiv;
  arrange(selmon);
}

static void
incrgaps(const Arg *arg)
{
  setgaps(
    selmon->gappoh + arg->i,
    selmon->gappov + arg->i,
    selmon->gappih + arg->i,
    selmon->gappiv + arg->i
  );
}

static void
incrigaps(const Arg *arg)
{
  setgaps(
    selmon->gappoh,
    selmon->gappov,
    selmon->gappih + arg->i,
    selmon->gappiv + arg->i
  );
}

static void
incrogaps(const Arg *arg)
{
  setgaps(
    selmon->gappoh + arg->i,
    selmon->gappov + arg->i,
    selmon->gappih,
    selmon->gappiv
  );
}

static void
incrohgaps(const Arg *arg)
{
  setgaps(
    selmon->gappoh + arg->i,
    selmon->gappov,
    selmon->gappih,
    selmon->gappiv
  );
}

static void
incrovgaps(const Arg *arg)
{
  setgaps(
    selmon->gappov,
    selmon->gappov + arg->i,
    selmon->gappih,
    selmon->gappiv
  );
}

static void
incrihgaps(const Arg *arg)
{
  setgaps(
    selmon->gappoh,
    selmon->gappov,
    selmon->gappih + arg->i,
    selmon->gappiv
  );
}

static void
incrivgaps(const Arg *arg)
{
  setgaps(
    selmon->gappoh,
    selmon->gappov,
    selmon->gappih,
    selmon->gappiv + arg->i
  );
}
