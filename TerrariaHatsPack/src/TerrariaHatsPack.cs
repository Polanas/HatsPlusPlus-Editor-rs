using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Reflection;
using Microsoft.Xna.Framework;
using Microsoft.Xna.Framework.Graphics;
using DuckGame;
using HatsPlusPlus;

[assembly: AssemblyTitle("TerrariaHatsPack | Client Mod")]
[assembly: AssemblyDescription("A bunch of Terratirs themed hats with animaetd wings and pets.")]
[assembly: AssemblyCompany("Polanas")]

[assembly: AssemblyCopyright("Copyright ©  2021")]
[assembly: AssemblyProduct("")]
[assembly: AssemblyTrademark("")]
[assembly: AssemblyCulture("")]
[assembly: AssemblyConfiguration("")]

[assembly: AssemblyVersion("1.0.0.0")]
[assembly: AssemblyFileVersion("1.0.0.0")]


namespace DuckGame
{
    public class TerrariaHatsPack : DisabledMod
    {
        public override Priority priority
        {
            get { return base.priority; }
        }

        protected override void OnPreInitialize()
        {
            HatsUtils.AddHatPath<TerrariaHatsPack>("hats");
            base.OnPreInitialize();
        }

        protected override void OnPostInitialize()
        {
            base.OnPostInitialize();
        }
    }
}
